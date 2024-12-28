use crate::{codegen::llvm::LLVMCodeGen, error::IoError, Result};
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
        let i8_ptr = self.context.i8_type().ptr_type(Default::default());

        // Register network functions
        let connect_type = i32_type.fn_type(&[i8_ptr.into()], false);
        let connect_fn = codegen
            .module
            .add_function("net_connect", connect_type, None);
        self.functions.insert("connect".to_string(), connect_fn);

        self.register_tcp_functions(codegen)?;
        self.register_udp_functions(codegen)?;

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

    fn register_network_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let socket_type = self.socket_type.unwrap();
        let addr_type = self.addr_type.unwrap();
        let i32_type = codegen.context.i32_type();
        let void_type = codegen.context.void_type();

        // Socket creation
        let socket_fn = codegen.module.add_function(
            "socket",
            socket_type.fn_type(&[i32_type.into(), i32_type.into(), i32_type.into()], false),
            None,
        );

        // Bind
        let bind_fn = codegen.module.add_function(
            "bind",
            i32_type.fn_type(
                &[
                    socket_type.into(),
                    addr_type.ptr_type(AddressSpace::default()).into(),
                ],
                false,
            ),
            None,
        );

        // Listen
        let listen_fn = codegen.module.add_function(
            "listen",
            i32_type.fn_type(&[socket_type.into(), i32_type.into()], false),
            None,
        );

        // Accept
        let accept_fn = codegen.module.add_function(
            "accept",
            socket_type.fn_type(
                &[
                    socket_type.into(),
                    addr_type.ptr_type(AddressSpace::default()).into(),
                ],
                false,
            ),
            None,
        );

        Ok(())
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
