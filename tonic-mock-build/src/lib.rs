use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[derive(Debug, Clone)]
pub struct Builder {
    build_client: bool,
    build_server: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            build_client: true,
            build_server: true,
        }
    }
}

pub fn configure() -> Builder {
    Builder::default()
}

pub fn compile_protos(
    proto: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    configure().compile(&[proto], &["."])
}

impl Builder {
    pub fn build_client(mut self, enabled: bool) -> Self {
        self.build_client = enabled;
        self
    }

    pub fn build_server(mut self, enabled: bool) -> Self {
        self.build_server = enabled;
        self
    }

    pub fn compile(
        self,
        protos: &[impl AsRef<std::path::Path>],
        includes: &[impl AsRef<std::path::Path>],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tonic = tonic_build::configure()
            .build_client(self.build_client)
            .build_server(self.build_server)
            .service_generator();

        let mut config = prost_build::Config::new();
        config.service_generator(Box::new(CombinedServiceGenerator {
            tonic,
            mock: self.build_server.then(MockServiceGenerator::default),
        }));
        config.compile_protos(protos, includes)?;
        Ok(())
    }
}

struct CombinedServiceGenerator {
    tonic: Box<dyn prost_build::ServiceGenerator>,
    mock: Option<MockServiceGenerator>,
}

impl prost_build::ServiceGenerator for CombinedServiceGenerator {
    fn generate(&mut self, service: prost_build::Service, buf: &mut String) {
        self.tonic.generate(service.clone(), buf);
        if let Some(mock) = self.mock.as_mut() {
            mock.generate(service, buf);
        }
    }

    fn finalize(&mut self, buf: &mut String) {
        self.tonic.finalize(buf);
        if let Some(mock) = self.mock.as_mut() {
            mock.finalize(buf);
        }
    }

    fn finalize_package(&mut self, package: &str, buf: &mut String) {
        self.tonic.finalize_package(package, buf);
        if let Some(mock) = self.mock.as_mut() {
            mock.finalize_package(package, buf);
        }
    }
}

#[derive(Default)]
struct MockServiceGenerator {
    services: TokenStream,
}

impl prost_build::ServiceGenerator for MockServiceGenerator {
    fn generate(&mut self, service: prost_build::Service, _buf: &mut String) {
        self.services.extend(generate_mock_service(&service));
    }

    fn finalize(&mut self, buf: &mut String) {
        if self.services.is_empty() {
            return;
        }

        let ast: syn::File = syn::parse2(self.services.clone()).expect("mock service code invalid");
        buf.push_str(&prettyplease::unparse(&ast));
        self.services = TokenStream::new();
    }
}

fn generate_mock_service(service: &prost_build::Service) -> TokenStream {
    let mock_name = format_ident!("Mock{}", service.name);
    let builder_name = format_ident!("Mock{}Builder", service.name);
    let server_mod = format_ident!("{}_server", service.name.to_snake_case());
    let mock_mod = format_ident!("{}_mock", service.name.to_snake_case());
    let trait_name = format_ident!("{}", service.name);
    let server_name = format_ident!("{}Server", service.name);

    let fields = service.methods.iter().filter_map(mock_field);
    let builder_fields = service.methods.iter().filter_map(builder_field);
    let build_fields = service.methods.iter().filter_map(|method| {
        let field = mock_field_ident(method);
        method_kind(method).map(|_| {
            quote! {
                #field: ::std::sync::Arc::new(::tokio::sync::RwLock::new(self.#field))
            }
        })
    });
    let builder_methods = service.methods.iter().filter_map(builder_method);
    let verify_blocks = service.methods.iter().filter_map(verify_block);
    let reset_blocks = service.methods.iter().filter_map(reset_block);
    let trait_items = service.methods.iter().map(trait_method);
    let associated_types = service.methods.iter().filter(|m| m.server_streaming).map(|method| {
        let ty = associated_stream_ident(method);
        let output = parse_mock_module_type(&method.output_type);
        quote! {
            type #ty = ::std::pin::Pin<
                Box<dyn ::futures::Stream<Item = Result<#output, ::tonic::Status>> + Send + 'static>
            >;
        }
    });

    quote! {
        pub mod #mock_mod {
            use super::*;

            #[derive(Debug, Default)]
            pub struct #builder_name {
                #(#builder_fields,)*
            }

            #[derive(Clone, Debug)]
            pub struct #mock_name {
                #(#fields,)*
                handle: ::std::sync::Arc<::tokio::sync::RwLock<Option<::tonic_mock::MockServerHandle>>>,
            }

            impl #builder_name {
                #(#builder_methods)*

                pub fn build(self) -> #mock_name {
                    #mock_name {
                        #(#build_fields,)*
                        handle: Default::default(),
                    }
                }

                pub async fn serve(self) -> #mock_name {
                    let service = self.build();
                    service.serve().await;
                    service
                }
            }

            impl #mock_name {
                pub fn builder() -> #builder_name {
                    #builder_name::default()
                }

                pub async fn serve(&self) {
                    let listener = ::tokio::net::TcpListener::bind("127.0.0.1:0")
                        .await
                        .expect("failed to bind tonic mock server listener");
                    let addr = listener
                        .local_addr()
                        .expect("failed to read tonic mock server listener address");
                    let incoming = ::tonic::transport::server::TcpIncoming::from_listener(listener, true, None)
                        .expect("failed to create tonic mock tcp incoming");
                    let (shutdown_tx, shutdown_rx) = ::tokio::sync::oneshot::channel();
                    let to_serve = self.clone();

                    ::tokio::spawn(async move {
                        let _ = ::tonic::transport::Server::builder()
                            .add_service(super::#server_mod::#server_name::new(to_serve))
                            .serve_with_incoming_shutdown(incoming, async move {
                                let _ = shutdown_rx.await;
                            })
                            .await;
                    });

                    *self.handle.write().await = Some(::tonic_mock::MockServerHandle::new(addr, shutdown_tx));
                }

                pub async fn endpoint(&self) -> Option<String> {
                    self.handle.read().await.as_ref().map(|handle| handle.endpoint())
                }

                pub async fn uri(&self) -> Option<String> {
                    self.endpoint().await
                }

                pub async fn reset(&self) {
                    #(#reset_blocks)*
                }

                pub async fn verify(&self) -> Result<(), ::tonic_mock::VerificationError> {
                    let mut errors = Vec::new();
                    #(#verify_blocks)*
                    if errors.is_empty() {
                        Ok(())
                    } else {
                        Err(::tonic_mock::VerificationError::new(errors))
                    }
                }

                pub async fn assert_verified(&self) {
                    if let Err(error) = self.verify().await {
                        panic!("{}", error);
                    }
                }
            }

            #[::tonic::async_trait]
            impl super::#server_mod::#trait_name for #mock_name {
                #(#associated_types)*
                #(#trait_items)*
            }
        }

    }
}

fn mock_field(method: &prost_build::Method) -> Option<TokenStream> {
    let field = mock_field_ident(method);
    let input = parse_mock_module_type(&method.input_type);
    let output = parse_mock_module_type(&method.output_type);
    match method_kind(method)? {
        MethodKind::Unary => Some(quote! {
            #field: ::std::sync::Arc<::tokio::sync::RwLock<Option<::tonic_mock::UnaryMethodMock<#input, #output>>>>
        }),
        MethodKind::ServerStreaming => Some(quote! {
            #field: ::std::sync::Arc<::tokio::sync::RwLock<Option<::tonic_mock::ServerStreamingMethodMock<#input, #output>>>>
        }),
    }
}

fn builder_field(method: &prost_build::Method) -> Option<TokenStream> {
    let field = mock_field_ident(method);
    let input = parse_mock_module_type(&method.input_type);
    let output = parse_mock_module_type(&method.output_type);
    match method_kind(method)? {
        MethodKind::Unary => Some(quote! {
            #field: Option<::tonic_mock::UnaryMethodMock<#input, #output>>
        }),
        MethodKind::ServerStreaming => Some(quote! {
            #field: Option<::tonic_mock::ServerStreamingMethodMock<#input, #output>>
        }),
    }
}

fn builder_method(method: &prost_build::Method) -> Option<TokenStream> {
    let method_name = format_ident!("mock_{}", method.name);
    let field = mock_field_ident(method);
    let input = parse_mock_module_type(&method.input_type);
    let output = parse_mock_module_type(&method.output_type);
    match method_kind(method)? {
        MethodKind::Unary => Some(quote! {
            pub fn #method_name(
                mut self,
                mock: ::tonic_mock::UnaryMethodMock<#input, #output>,
            ) -> Self {
                self.#field = Some(mock);
                self
            }
        }),
        MethodKind::ServerStreaming => Some(quote! {
            pub fn #method_name(
                mut self,
                mock: ::tonic_mock::ServerStreamingMethodMock<#input, #output>,
            ) -> Self {
                self.#field = Some(mock);
                self
            }
        }),
    }
}

fn verify_block(method: &prost_build::Method) -> Option<TokenStream> {
    method_kind(method)?;
    let field = mock_field_ident(method);
    let name = method.name.clone();
    Some(quote! {
        if let Some(mock) = self.#field.read().await.as_ref() {
            if let Err(error) = mock.verify_method(#name) {
                errors.extend(error.messages().iter().cloned());
            }
        }
    })
}

fn reset_block(method: &prost_build::Method) -> Option<TokenStream> {
    method_kind(method)?;
    let field = mock_field_ident(method);
    Some(quote! {
        if let Some(mock) = self.#field.write().await.as_mut() {
            mock.reset();
        }
    })
}

fn trait_method(method: &prost_build::Method) -> TokenStream {
    let fn_name = format_ident!("{}", method.name);
    let input = parse_mock_module_type(&method.input_type);
    let output = parse_mock_module_type(&method.output_type);
    let field = mock_field_ident(method);

    match (method.client_streaming, method.server_streaming) {
        (false, false) => quote! {
            async fn #fn_name(
                &self,
                request: ::tonic::Request<#input>,
            ) -> Result<::tonic::Response<#output>, ::tonic::Status> {
                if let Some(mock) = self.#field.read().await.as_ref() {
                    mock.process_request(request)
                } else {
                    Err(::tonic::Status::unimplemented(concat!(stringify!(#fn_name), " is not implemented")))
                }
            }
        },
        (false, true) => {
            let stream_ty = associated_stream_ident(method);
            quote! {
                async fn #fn_name(
                    &self,
                    request: ::tonic::Request<#input>,
                ) -> Result<::tonic::Response<Self::#stream_ty>, ::tonic::Status> {
                    if let Some(mock) = self.#field.read().await.as_ref() {
                        mock.process_request(request)
                            .map(|response| response.map(|stream| Box::pin(stream) as Self::#stream_ty))
                    } else {
                        Err(::tonic::Status::unimplemented(concat!(stringify!(#fn_name), " is not implemented")))
                    }
                }
            }
        }
        (true, false) => quote! {
            async fn #fn_name(
                &self,
                _request: ::tonic::Request<::tonic::Streaming<#input>>,
            ) -> Result<::tonic::Response<#output>, ::tonic::Status> {
                Err(::tonic::Status::unimplemented(concat!(stringify!(#fn_name), " is not implemented")))
            }
        },
        (true, true) => {
            let stream_ty = associated_stream_ident(method);
            quote! {
                async fn #fn_name(
                    &self,
                    _request: ::tonic::Request<::tonic::Streaming<#input>>,
                ) -> Result<::tonic::Response<Self::#stream_ty>, ::tonic::Status> {
                    Err(::tonic::Status::unimplemented(concat!(stringify!(#fn_name), " is not implemented")))
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum MethodKind {
    Unary,
    ServerStreaming,
}

fn method_kind(method: &prost_build::Method) -> Option<MethodKind> {
    match (method.client_streaming, method.server_streaming) {
        (false, false) => Some(MethodKind::Unary),
        (false, true) => Some(MethodKind::ServerStreaming),
        _ => None,
    }
}

fn mock_field_ident(method: &prost_build::Method) -> syn::Ident {
    format_ident!("{}_mock", method.name)
}

fn associated_stream_ident(method: &prost_build::Method) -> syn::Ident {
    format_ident!("{}Stream", method.proto_name.to_upper_camel_case())
}

fn parse_type(ty: &str) -> syn::Type {
    syn::parse_str(ty).unwrap_or_else(|_| panic!("invalid generated Rust type: {}", ty))
}

fn parse_mock_module_type(ty: &str) -> syn::Type {
    let ty = ty
        .strip_prefix("super::")
        .map(|rest| format!("super::super::{}", rest))
        .unwrap_or_else(|| ty.to_owned());
    parse_type(&ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost_build::ServiceGenerator;

    fn render_service(build_client: bool, build_server: bool) -> String {
        let tonic = tonic_build::configure()
            .build_client(build_client)
            .build_server(build_server)
            .service_generator();
        let mut generator = CombinedServiceGenerator {
            tonic,
            mock: build_server.then(MockServiceGenerator::default),
        };
        let mut buf = String::new();
        generator.generate(test_service(), &mut buf);
        generator.finalize(&mut buf);
        buf
    }

    fn test_service() -> prost_build::Service {
        prost_build::Service {
            name: "Example".to_owned(),
            proto_name: "Example".to_owned(),
            package: "fixtures".to_owned(),
            comments: Default::default(),
            methods: vec![prost_build::Method {
                name: "get".to_owned(),
                proto_name: "Get".to_owned(),
                comments: Default::default(),
                input_type: "Request".to_owned(),
                output_type: "Reply".to_owned(),
                input_proto_type: ".fixtures.Request".to_owned(),
                output_proto_type: ".fixtures.Reply".to_owned(),
                options: Default::default(),
                client_streaming: false,
                server_streaming: false,
            }],
            options: Default::default(),
        }
    }

    #[test]
    fn emits_mocks_when_server_generation_is_enabled_without_clients() {
        let generated = render_service(false, true);

        assert!(generated.contains("pub mod example_server"));
        assert!(generated.contains("pub mod example_mock"));
        assert!(!generated.contains("pub mod example_client"));
    }

    #[test]
    fn skips_mocks_when_server_generation_is_disabled() {
        let generated = render_service(true, false);

        assert!(generated.contains("pub mod example_client"));
        assert!(!generated.contains("pub mod example_server"));
        assert!(!generated.contains("pub mod example_mock"));
    }
}
