use crate::{codegen::llvm::LLVMCodeGen, error::IoError, Result};
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::AddressSpace;
use inkwell::{context::Context, types::BasicType, values::FunctionValue};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::TcpStream,
};
use tokio::net::TcpStream as AsyncTcpStream;

pub struct NetworkError(String);

impl From<io::Error> for NetworkError {
    fn from(error: io::Error) -> Self {
        NetworkError(error.to_string())
    }
}

impl From<NetworkError> for IoError {
    fn from(err: NetworkError) -> Self {
        IoError::runtime_error(err.0)
    }
}

pub struct NetworkConnection {
    stream: TcpStream,
}

impl NetworkConnection {
    pub fn connect(address: &str) -> Result<Self> {
        let stream = TcpStream::connect(address).map_err(IoError::from)?;
        Ok(Self { stream })
    }

    pub fn send(&mut self, data: &[u8]) -> Result<usize> {
        self.stream.write(data).map_err(IoError::from)
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.stream.read(buffer).map_err(IoError::from)
    }
}

pub struct AsyncNetworkConnection {
    stream: AsyncTcpStream,
}

impl AsyncNetworkConnection {
    pub async fn connect(address: &str) -> Result<Self> {
        let stream = AsyncTcpStream::connect(address)
            .await
            .map_err(IoError::from)?;
        Ok(Self { stream })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        self.stream.try_write(data).map_err(IoError::from)
    }

    pub async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.stream.try_read(buffer).map_err(IoError::from)
    }
}

pub struct NetworkModule<'ctx> {
    functions: HashMap<String, FunctionValue<'ctx>>,
    context: &'ctx Context,
    socket_type: Option<inkwell::types::StructType<'ctx>>,
    addr_type: Option<inkwell::types::StructType<'ctx>>,
}

impl<'ctx> NetworkModule<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            functions: HashMap::new(),
            context,
            socket_type: None,
            addr_type: None,
        }
    }

    pub fn connect(&mut self, _address: &str) -> Result<()> {
        Ok(())
    }

    pub fn send(&mut self, data: &[u8]) -> Result<usize> {
        Ok(data.len())
    }

    pub fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.functions.get(name).copied()
    }

    pub fn generate_bindings(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        self.register_network_types(codegen)?;
        self.register_network_functions(codegen)?;
        Ok(())
    }

    pub fn register_network_types(&self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let socket_type = self.context.i32_type().as_basic_type_enum();
        codegen.register_type("socket", socket_type)?;
        Ok(())
    }

    pub fn register_network_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i32_type = self.context.i32_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());

        let metadata_types: Vec<BasicMetadataTypeEnum<'ctx>> =
            vec![i32_type.into(), i32_type.into(), i32_type.into()];

        let socket_fn_type = i32_type.fn_type(&metadata_types, false);

        let _ = codegen.module.add_function("socket", socket_fn_type, None);

        self.register_tcp_functions(codegen)?;
        self.register_udp_functions(codegen)?;
        self.register_dns_functions(codegen)?;

        Ok(())
    }

    pub fn initialize(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        self.register_socket_type(codegen)?;
        self.register_addr_type(codegen)?;
        self.register_network_functions(codegen)?;
        Ok(())
    }

    fn register_socket_type(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr = codegen.context.ptr_type(AddressSpace::default());
        let i32_type = codegen.context.i32_type();

        let socket_type = codegen.context.struct_type(
            &[
                i32_type.into(), // fd
                i32_type.into(), // family
                i32_type.into(), // type
                i32_type.into(), // protocol
            ],
            false,
        );

        self.socket_type = Some(socket_type);
        Ok(())
    }

    fn register_addr_type(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i32_type = codegen.context.i32_type();
        let i8_type = codegen.context.i8_type();

        let addr_type = codegen.context.struct_type(
            &[
                i32_type.into(),               // family
                i8_type.array_type(14).into(), // data
            ],
            false,
        );

        self.addr_type = Some(addr_type);
        Ok(())
    }

    fn register_tls_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr = codegen.context.ptr_type(AddressSpace::default());
        let i32_type = codegen.context.i32_type();

        // TLS initialization
        let init_tls_fn_type = i32_type.fn_type(&[i8_ptr.into()], false);
        codegen
            .module
            .add_function("tls_init", init_tls_fn_type, None);

        // TLS connect
        let connect_tls_fn_type = i32_type.fn_type(&[i32_type.into(), i8_ptr.into()], false);
        codegen
            .module
            .add_function("tls_connect", connect_tls_fn_type, None);

        Ok(())
    }

    fn register_dns_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr = codegen.context.ptr_type(AddressSpace::default());

        // DNS resolve
        let resolve_fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
        codegen
            .module
            .add_function("dns_resolve", resolve_fn_type, None);

        // DNS reverse lookup
        let reverse_fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
        codegen
            .module
            .add_function("dns_reverse", reverse_fn_type, None);

        Ok(())
    }

    fn register_tcp_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(Default::default());

        let connect_fn_type = i32_type.fn_type(&[i8_ptr_type.into(), i32_type.into()], false);
        let connect_fn = codegen
            .module
            .add_function("tcp_connect", connect_fn_type, None);
        self.functions.insert("tcp_connect".to_string(), connect_fn);

        let send_fn_type = i32_type.fn_type(
            &[
                i32_type.into(),    // socket fd
                i8_ptr_type.into(), // buffer
                i32_type.into(),    // length
            ],
            false,
        );
        let send_fn = codegen.module.add_function("tcp_send", send_fn_type, None);
        self.functions.insert("tcp_send".to_string(), send_fn);
        Ok(())
    }

    fn register_udp_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(Default::default());

        let socket_fn_type = i32_type.fn_type(&[], false);
        let socket_fn = codegen
            .module
            .add_function("udp_socket", socket_fn_type, None);
        self.functions.insert("udp_socket".to_string(), socket_fn);

        let sendto_fn_type = i32_type.fn_type(
            &[
                i32_type.into(),    // socket fd
                i8_ptr_type.into(), // buffer
                i32_type.into(),    // length
                i8_ptr_type.into(), // destination address
                i32_type.into(),    // address length
            ],
            false,
        );
        let sendto_fn = codegen
            .module
            .add_function("udp_sendto", sendto_fn_type, None);
        self.functions.insert("udp_sendto".to_string(), sendto_fn);
        Ok(())
    }

    fn register_http_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr_type = self.context.i8_type().ptr_type(Default::default());
        let get_fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        let get_fn = codegen.module.add_function("http_get", get_fn_type, None);
        self.functions.insert("http_get".to_string(), get_fn);

        let post_fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let post_fn = codegen.module.add_function("http_post", post_fn_type, None);
        self.functions.insert("http_post".to_string(), post_fn);
        Ok(())
    }
}

pub fn open_connection(address: &str) -> Result<()> {
    // Parse address into host and port
    let parts: Vec<&str> = address.split(':').collect();
    if parts.len() != 2 {
        return Err(IoError::runtime_error("Invalid address format"));
    }

    let host = parts[0];
    let port = parts[1]
        .parse::<u16>()
        .map_err(|_| IoError::runtime_error("Invalid port number"))?;

    // Try to establish TCP connection
    match TcpStream::connect((host, port)) {
        Ok(_stream) => {
            // Connection successful
            Ok(())
        }
        Err(e) => Err(IoError::runtime_error(&format!("Failed to connect: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_sync_connection() {
        let server = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).unwrap();
            assert_eq!(&buffer[..n], b"hello");
        });

        let mut client = NetworkConnection::connect("127.0.0.1:8080").unwrap();
        client.send(b"hello").unwrap();
        server.join().unwrap();
    }

    #[tokio::test]
    async fn test_async_connection() {
        let server = tokio::spawn(async {
            let listener = AsyncTcpListener::bind("127.0.0.1:8081").await.unwrap();
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 1024];
            let n = stream.try_read(&mut buffer).unwrap();
            assert_eq!(&buffer[..n], b"hello async");
        });

        let mut client = AsyncNetworkConnection::connect("127.0.0.1:8081")
            .await
            .unwrap();
        client.send(b"hello async").await.unwrap();
        server.await.unwrap();
    }
}
