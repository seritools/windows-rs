use super::*;

pub fn writer(writer: &Writer, def: TypeDef) -> TokenStream {
    if def.has_attribute("ApiContractAttribute") {
        return quote! {};
    }

    if type_def_is_handle(def) {
        return handles::writer(writer, def);
    }

    gen_struct_with_name(writer, def, def.name(), &Cfg::default())
}

fn gen_struct_with_name(writer: &Writer, def: TypeDef, struct_name: &str, cfg: &Cfg) -> TokenStream {
    let name = to_ident(struct_name);

    if def.fields().next().is_none() {
        let mut tokens = quote! {
            #[repr(C)]
            pub struct #name(pub u8);
            impl ::core::marker::Copy for #name {}
            impl ::core::clone::Clone for #name {
                fn clone(&self) -> Self {
                    *self
                }
            }
        };
        if !writer.sys {
            tokens.combine(&quote! {
                impl ::windows_core::TypeKind for #name {
                    type TypeKind = ::windows_core::CopyType;
                }
            });
        }
        return tokens;
    }

    let flags = def.flags();
    let cfg = cfg.union(&type_def_cfg(def, &[]));

    let repr = if let Some(layout) = def.class_layout() {
        let packing = Literal::usize_unsuffixed(layout.packing_size());
        quote! { #[repr(C, packed(#packing))] }
    } else {
        quote! { #[repr(C)] }
    };

    let fields = def.fields().map(|f| {
        let name = to_ident(f.name());
        let ty = f.ty(Some(def));

        if f.flags().contains(FieldAttributes::Literal) {
            quote! {}
        } else if !writer.sys && flags.contains(TypeAttributes::ExplicitLayout) && !field_is_copyable(f, def) {
            let ty = writer.type_default_name(&ty);
            quote! { pub #name: ::std::mem::ManuallyDrop<#ty>, }
        } else if !writer.sys && !flags.contains(TypeAttributes::WindowsRuntime) && !field_is_blittable(f, def) {
            if let Type::Win32Array(ty, len) = ty {
                let ty = writer.type_default_name(&ty);
                quote! { pub #name: [::std::mem::ManuallyDrop<#ty>; #len], }
            } else {
                let ty = writer.type_default_name(&ty);
                quote! { pub #name: ::std::mem::ManuallyDrop<#ty>, }
            }
        } else {
            let ty = writer.type_default_name(&ty);
            quote! { pub #name: #ty, }
        }
    });

    let struct_or_union = if flags.contains(TypeAttributes::ExplicitLayout) {
        quote! { union }
    } else {
        quote! { struct }
    };

    let doc = writer.cfg_doc(&cfg);
    let features = writer.cfg_features(&cfg);

    let mut tokens = quote! {
        #repr
        #doc
        #features
        pub #struct_or_union #name {#(#fields)*}
    };

    tokens.combine(&gen_struct_constants(writer, def, &name, &cfg));
    tokens.combine(&gen_copy_clone(writer, def, &name, &cfg));
    tokens.combine(&gen_debug(writer, def, &name, &cfg));
    tokens.combine(&gen_windows_traits(writer, def, &name, &cfg));
    tokens.combine(&gen_compare_traits(writer, def, &name, &cfg));

    if !writer.sys {
        tokens.combine(&quote! {
            #features
            impl ::core::default::Default for #name {
                fn default() -> Self {
                    unsafe { ::core::mem::zeroed() }
                }
            }
        });
    }

    for (index, nested_type) in writer.reader.nested_types(def).enumerate() {
        let nested_name = format!("{struct_name}_{index}");
        tokens.combine(&gen_struct_with_name(writer, nested_type, &nested_name, &cfg));
    }

    tokens
}

fn gen_windows_traits(writer: &Writer, def: TypeDef, name: &TokenStream, cfg: &Cfg) -> TokenStream {
    if writer.sys {
        quote! {}
    } else {
        let features = writer.cfg_features(cfg);
        let is_copy = type_def_is_blittable(def);

        let type_kind = if is_copy {
            quote! { CopyType }
        } else {
            quote! { ValueType }
        };

        let mut tokens = quote! {
            #features
            impl ::windows_core::TypeKind for #name {
                type TypeKind = ::windows_core::#type_kind;
            }
        };

        if def.flags().contains(TypeAttributes::WindowsRuntime) {
            let signature = Literal::byte_string(type_def_signature(def, &[]).as_bytes());

            tokens.combine(&quote! {
                #features
                impl ::windows_core::RuntimeType for #name {
                    const SIGNATURE: ::windows_core::imp::ConstBuffer = ::windows_core::imp::ConstBuffer::from_slice(#signature);
                }
            });
        }

        tokens
    }
}

fn gen_compare_traits(writer: &Writer, def: TypeDef, name: &TokenStream, cfg: &Cfg) -> TokenStream {
    let features = writer.cfg_features(cfg);

    if writer.sys || type_def_has_explicit_layout(def) || type_def_has_packing(def) || type_def_has_callback(def) {
        quote! {}
    } else {
        let fields = def.fields().filter_map(|f| {
            let name = to_ident(f.name());
            if f.flags().contains(FieldAttributes::Literal) {
                None
            } else {
                Some(quote! { self.#name == other.#name })
            }
        });

        quote! {
            #features
            impl ::core::cmp::PartialEq for #name {
                fn eq(&self, other: &Self) -> bool {
                    #(#fields)&&*
                }
            }
            #features
            impl ::core::cmp::Eq for #name {}
        }
    }
}

fn gen_debug(writer: &Writer, def: TypeDef, ident: &TokenStream, cfg: &Cfg) -> TokenStream {
    if writer.sys || type_def_has_explicit_layout(def) || type_def_has_packing(def) {
        quote! {}
    } else {
        let name = ident.as_str();
        let features = writer.cfg_features(cfg);

        let fields = def.fields().filter_map(|f| {
            if f.flags().contains(FieldAttributes::Literal) {
                None
            } else {
                let name = f.name();
                let ident = to_ident(name);
                let ty = f.ty(Some(def));
                if type_has_callback(&ty) {
                    None
                } else {
                    Some(quote! { .field(#name, &self.#ident) })
                }
            }
        });

        quote! {
            #features
            impl ::core::fmt::Debug for #ident {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct(#name) #(#fields)* .finish()
                }
            }
        }
    }
}

fn gen_copy_clone(writer: &Writer, def: TypeDef, name: &TokenStream, cfg: &Cfg) -> TokenStream {
    let features = writer.cfg_features(cfg);

    if writer.sys || type_def_is_copyable(def) {
        quote! {
            #features
            impl ::core::marker::Copy for #name {}
            #features
            impl ::core::clone::Clone for #name {
                fn clone(&self) -> Self {
                    *self
                }
            }
        }
    } else if def.class_layout().is_some() {
        // Don't support copy/clone of packed structs: https://github.com/rust-lang/rust/issues/82523
        quote! {}
    } else if !def.flags().contains(TypeAttributes::WindowsRuntime) {
        quote! {
            #features
            impl ::core::clone::Clone for #name {
                fn clone(&self) -> Self {
                    unsafe { ::core::mem::transmute_copy(self) }
                }
            }
        }
    } else {
        let fields = def.fields().map(|f| {
            let name = to_ident(f.name());
            if f.flags().contains(FieldAttributes::Literal) {
                quote! {}
            } else if field_is_blittable(f, def) {
                quote! { #name: self.#name }
            } else {
                quote! { #name: self.#name.clone() }
            }
        });

        quote! {
            #features
            impl ::core::clone::Clone for #name {
                fn clone(&self) -> Self {
                    Self { #(#fields),* }
                }
            }
        }
    }
}

fn gen_struct_constants(writer: &Writer, def: TypeDef, struct_name: &TokenStream, cfg: &Cfg) -> TokenStream {
    let features = writer.cfg_features(cfg);

    let constants = def.fields().filter_map(|f| {
        if f.flags().contains(FieldAttributes::Literal) {
            if let Some(constant) = f.constant() {
                let name = to_ident(f.name());
                let value = writer.typed_value(&constant.value());

                return Some(quote! {
                    pub const #name: #value;
                });
            }
        }

        None
    });

    let mut tokens = quote! { #(#constants)* };

    if !tokens.is_empty() {
        tokens = quote! {
            #features
            impl #struct_name {
                #tokens
            }
        };
    }

    tokens
}
