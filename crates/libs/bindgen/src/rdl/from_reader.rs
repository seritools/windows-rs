use super::*;
use crate::tokens::{quote, to_ident, TokenStream};
use crate::{rdl, Error, Result, Tree};
use metadata::*;

pub fn from_reader(reader: &'static metadata::Reader, mut config: std::collections::BTreeMap<&str, &str>, output: &str) -> Result<()> {
    let dialect = match config.remove("type") {
        Some("winrt") => Dialect::WinRT,
        Some("win32") => Dialect::Win32,
        _ => return Err(Error::new("configuration value `type` must be `win32` or `winrt`")),
    };

    let mut writer = Writer::new(reader, output, dialect);

    // TODO: be sure to use the same "split" key for winmd splitting.
    // May also want to support split=N similar to the way MIDLRT supports winmd splitting
    // at different nesting levels.
    writer.split = config.remove("split").is_some();

    if let Some((key, _)) = config.first_key_value() {
        return Err(Error::new(&format!("invalid configuration value `{key}`")));
    }

    if writer.split {
        gen_split(&writer)
    } else {
        gen_file(&writer)
    }
}

fn gen_split(writer: &Writer) -> Result<()> {
    let tree = Tree::new(writer.reader);
    let directory = crate::directory(&writer.output);

    // TODO: parallelize
    for tree in tree.flatten() {
        let tokens = writer.tree(tree);

        if !tokens.is_empty() {
            let output = format!("{directory}/{}.rdl", tree.namespace);
            writer.write_to_file(&output, tokens)?;
        }
    }

    Ok(())
}

fn gen_file(writer: &Writer) -> Result<()> {
    let tree = Tree::new(writer.reader);
    let tokens = writer.tree(&tree);
    writer.write_to_file(&writer.output, tokens)
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Dialect {
    Win32,
    WinRT,
}

struct Writer {
    reader: &'static metadata::Reader,
    namespace: &'static str,
    dialect: Dialect,
    split: bool,
    output: String,
}

impl Writer {
    fn new(reader: &'static metadata::Reader, output: &str, dialect: Dialect) -> Self {
        Self { reader, namespace: "", output: output.to_string(), dialect, split: false }
    }

    fn with_namespace(&self, namespace: &'static str) -> Self {
        Self { reader: self.reader, namespace, dialect: self.dialect, output: self.output.clone(), split: self.split }
    }

    fn write_to_file(&self, output: &str, tokens: TokenStream) -> Result<()> {
        let dialect = match self.dialect {
            Dialect::Win32 => quote! { #![win32] },
            Dialect::WinRT => quote! { #![winrt] },
        };

        let tokens = quote! {
            #dialect
            #tokens
        };

        let file = rdl::File::parse_str(&tokens.into_string())?;
        crate::write_to_file(output, file.fmt())
        //crate::write_to_file(output, tokens.into_string())
    }

    fn tree(&self, tree: &Tree) -> TokenStream {
        let items = self.items(tree);

        if self.split {
            let mut tokens = items;

            if !tokens.is_empty() {
                for name in tree.namespace.rsplit('.').map(to_ident) {
                    tokens = quote! {
                        mod #name {
                            #tokens
                        }
                    };
                }
            }

            tokens
        } else {
            let name = to_ident(tree.namespace.rsplit_once('.').map_or(tree.namespace, |(_, name)| name));

            let modules = tree.nested.values().map(|tree| self.with_namespace(tree.namespace).tree(tree));

            if tree.namespace.is_empty() {
                quote! {
                    #(#modules)*
                    #items
                }
            } else {
                quote! {
                    mod #name {
                        #(#modules)*
                        #items
                    }
                }
            }
        }
    }

    fn items(&self, tree: &Tree) -> TokenStream {
        let mut functions = vec![];
        let mut constants = vec![];
        let mut types = vec![];

        if !tree.namespace.is_empty() {
            for item in self.reader.namespace_items(tree.namespace).filter(|item| match item {
                metadata::Item::Type(def) => {
                    let winrt = def.flags().contains(metadata::TypeAttributes::WindowsRuntime);
                    match self.dialect {
                        Dialect::Win32 => !winrt,
                        Dialect::WinRT => winrt,
                    }
                }
                metadata::Item::Fn(_, _) | metadata::Item::Const(_) => self.dialect == Dialect::Win32,
            }) {
                match item {
                    metadata::Item::Type(def) => types.push(self.type_def(def)),
                    metadata::Item::Const(field) => constants.push(self.constant(field)),
                    metadata::Item::Fn(method, namespace) => functions.push(self.function(method, namespace)),
                }
            }
        }

        quote! {
            #(#functions)*
            #(#constants)*
            #(#types)*
        }
    }

    fn function(&self, def: metadata::MethodDef, _namespace: &str) -> TokenStream {
        let name = to_ident(def.name());
        quote! { fn #name(); }
    }

    fn constant(&self, def: metadata::Field) -> TokenStream {
        let name = to_ident(def.name());
        quote! { const #name: i32 = 0; }
    }

    fn type_def(&self, def: metadata::TypeDef) -> TokenStream {
        if let Some(extends) = def.extends() {
            if extends.namespace == "System" {
                if extends.name == "Enum" {
                    self.enum_def(def)
                } else if extends.name == "ValueType" {
                    self.struct_def(def)
                } else if extends.name == "MulticastDelegate" {
                    self.delegate_def(def)
                } else {
                    self.class_def(def)
                }
            } else {
                self.class_def(def)
            }
        } else {
            self.interface_def(def)
        }
    }

    fn enum_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(def.name());

        quote! {
            struct #name {

            }
        }
    }

    fn struct_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(def.name());

        let fields = def.fields().map(|field| {
            let name = to_ident(field.name());
            let ty = self.ty(&field.ty(Some(def)));
            quote! {
                #name: #ty
            }
        });

        quote! {
            struct #name {
                #(#fields),*
            }
        }
    }

    fn delegate_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(def.name());

        quote! {
            struct #name {

            }
        }
    }

    fn class_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(def.name());
        let implements = self.implements(def, &[]);

        quote! {
            class #name #implements;
        }
    }

    fn interface_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(def.name());
        let generics = &metadata::type_def_generics(def);
        let implements = self.implements(def, generics);

        let methods = def.methods().map(|method| {
            let name = to_ident(method.name());

            // TODO: use reader.method_def_signature instead
            let signature = metadata::method_def_signature(def.namespace(), method, generics);

            let return_type = self.return_type(&signature.return_type);

            let params = signature.params.iter().map(|param| {
                let name = to_ident(param.def.name());
                let ty = self.ty(&param.ty);
                quote! { #name: #ty }
            });

            quote! {
                fn #name(#(#params),*) #return_type;
            }
        });

        let generics = self.generics(generics);

        quote! {
            interface #name #generics #implements {
                #(#methods)*
            }
        }
    }

    fn generics(&self, generics: &[metadata::Type]) -> TokenStream {
        if generics.is_empty() {
            quote! {}
        } else {
            let generics = generics.iter().map(|generic| self.ty(generic));

            quote! { <#(#generics),*>}
        }
    }

    fn implements(&self, def: metadata::TypeDef, generics: &[metadata::Type]) -> TokenStream {
        let mut types = Vec::<TokenStream>::new();

        // TODO: if a winrt composable class then start with base
        // TODO: then list default interface first
        // Then everything else

        for interface in type_def_interfaces(def, generics) {
            if interface.kind == InterfaceKind::Default {
                types.insert(0, self.ty(&interface.ty));
            } else {
                types.push(self.ty(&interface.ty));
            }
        }

        if let Some(type_name) = def.extends() {
            if type_name != metadata::TypeName::Object {
                let namespace = self.namespace(type_name.namespace);
                let name = to_ident(type_name.name);
                // TODO: ideally the "class" contextual keyword wouldn't be needed here
                // but currently there's no way to tell the base class apart from a required interface.
                types.insert(0, quote! { class #namespace #name });
            }
        }

        if types.is_empty() {
            quote! {}
        } else {
            quote! { : #(#types),* }
        }
    }

    fn return_type(&self, ty: &metadata::Type) -> TokenStream {
        match ty {
            metadata::Type::Void => quote! {},
            _ => {
                let ty = self.ty(ty);
                quote! { -> #ty }
            }
        }
    }

    fn ty(&self, ty: &metadata::Type) -> TokenStream {
        match ty {
            metadata::Type::Void => quote! { ::core::ffi::c_void },
            metadata::Type::Bool => quote! { bool },
            metadata::Type::Char => quote! { u16 },
            metadata::Type::I8 => quote! { i8 },
            metadata::Type::U8 => quote! { u8 },
            metadata::Type::I16 => quote! { i16 },
            metadata::Type::U16 => quote! { u16 },
            metadata::Type::I32 => quote! { i32 },
            metadata::Type::U32 => quote! { u32 },
            metadata::Type::I64 => quote! { i64 },
            metadata::Type::U64 => quote! { u64 },
            metadata::Type::F32 => quote! { f32 },
            metadata::Type::F64 => quote! { f64 },
            metadata::Type::ISize => quote! { isize },
            metadata::Type::USize => quote! { usize },

            // TODO: dialect-specific keywords for "well-known types" that don't map to metadata in all cases.
            metadata::Type::String => quote! { HSTRING },
            metadata::Type::HRESULT => quote! { HRESULT },
            metadata::Type::GUID => quote! { GUID },
            metadata::Type::IInspectable => quote! { IInspectable },
            metadata::Type::IUnknown => quote! { IUnknown },

            metadata::Type::TypeDef(def, generics) => {
                let namespace = self.namespace(def.namespace());
                let name = to_ident(def.name());
                if generics.is_empty() {
                    quote! { #namespace #name }
                } else {
                    let generics = generics.iter().map(|ty| self.ty(ty));
                    quote! { #namespace #name<#(#generics,)*> }
                }
            }

            metadata::Type::TypeRef(type_name) => {
                let namespace = self.namespace(type_name.namespace);
                let name = to_ident(type_name.name);
                quote! { #namespace #name }
            }

            metadata::Type::GenericParam(generic) => generic.name().into(),
            metadata::Type::WinrtArray(ty) => self.ty(ty),
            metadata::Type::WinrtArrayRef(ty) => self.ty(ty),
            metadata::Type::ConstRef(ty) => self.ty(ty),
            metadata::Type::MutPtr(ty, _pointers) => self.ty(ty),
            metadata::Type::ConstPtr(ty, _pointers) => self.ty(ty),
            metadata::Type::Win32Array(ty, _len) => self.ty(ty),
            // TODO: these types should just be regular metadata type defs
            metadata::Type::PSTR => quote! { PSTR },
            metadata::Type::PWSTR => quote! { PWSTR },
            metadata::Type::PCSTR => quote! { PCSTR },
            metadata::Type::PCWSTR => quote! { PCWSTR },
            metadata::Type::BSTR => quote! { BSTR },
            metadata::Type::PrimitiveOrEnum(_, ty) => self.ty(ty),
            rest => unimplemented!("{rest:?}"),
        }
    }

    fn namespace(&self, namespace: &str) -> TokenStream {
        // TODO: handle nested structs?
        if namespace.is_empty() || self.namespace == namespace {
            quote! {}
        } else {
            // TODO: problem with making relative paths here is that we don't have the context to disambiguate

            // let mut relative = self.namespace.split('.').peekable();
            // let mut namespace = namespace.split('.').peekable();
            // let mut related = false;

            // while relative.peek() == namespace.peek() {
            //     related = true;

            //     if relative.next().is_none() {
            //         break;
            //     }

            //     namespace.next();
            // }

            // let mut tokens = TokenStream::new();

            // if related {
            //     for _ in 0..relative.count() {
            //         tokens.push_str("super::");
            //     }
            // }

            // for namespace in namespace {
            //     tokens.push_str(namespace);
            //     tokens.push_str("::");
            // }

            // tokens

            // TODO: so instead we just gen it out in full

            let mut tokens = TokenStream::new();

            for namespace in namespace.split('.') {
                tokens.push_str(namespace);
                tokens.push_str("::");
            }

            tokens
        }
    }
}
