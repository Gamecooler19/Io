use std::sync::Arc;
use tokio::sync::broadcast;
use crate::{Result, error::IoError};
use inkwell::values::FunctionValue;

pub struct SubscriptionManager<'ctx> {
    context: &'ctx inkwell::context::Context,
    subscribe_fn: FunctionValue<'ctx>,
    publish_fn: FunctionValue<'ctx>,
    channels: Arc<parking_lot::RwLock<HashMap<String, broadcast::Sender<Vec<u8>>>>>,
}

impl<'ctx> SubscriptionManager<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Result<Self> {
        let subscribe_type = context.opaque_struct_type("Subscription");
        let channels = Arc::new(parking_lot::RwLock::new(HashMap::new()));

        let subscribe_fn = Self::create_subscribe_function(context, subscribe_type)?;
        let publish_fn = Self::create_publish_function(context)?;

        Ok(Self {
            context,
            subscribe_fn,
            publish_fn,
            channels,
        })
    }

    fn create_subscribe_function(
        context: &'ctx inkwell::context::Context,
        subscribe_type: inkwell::types::StructType<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let fn_type = subscribe_type.fn_type(&[
            context.i8_ptr_type().into(), // Topic
            context.i8_ptr_type().into(), // Callback function
        ], false);

        Ok(context.module().add_function("subscribe", fn_type, None))
    }

    pub async fn subscribe(&self, topic: &str) -> Result<broadcast::Receiver<Vec<u8>>> {
        let channels = self.channels.read();
        if let Some(sender) = channels.get(topic) {
            Ok(sender.subscribe())
        } else {
            let (sender, receiver) = broadcast::channel(100);
            self.channels.write().insert(topic.to_string(), sender);
            Ok(receiver)
        }
    }

    pub async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<()> {
        if let Some(sender) = self.channels.read().get(topic) {
            sender.send(data).map_err(|e| {
                IoError::runtime_error(format!("Failed to publish message: {}", e))
            })?;
        }
        Ok(())
    }

    pub fn generate_bindings(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        self.generate_subscribe_binding(codegen)?;
        self.generate_publish_binding(codegen)?;
        Ok(())
    }

    pub fn add_subscription_filter(
        &mut self,
        topic: &str,
        filter: impl Fn(&[u8]) -> bool + Send + Sync + 'static,
    ) -> Result<()> {
        let mut filters = self.filters.write();
        filters.insert(topic.to_string(), Arc::new(filter));
        Ok(())
    }

    pub fn add_subscription_transformer(
        &mut self,
        topic: &str,
        transformer: impl Fn(Vec<u8>) -> Result<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<()> {
        let mut transformers = self.transformers.write();
        transformers.insert(topic.to_string(), Arc::new(transformer));
        Ok(())
    }

    pub async fn subscribe_with_backpressure(
        &self,
        topic: &str,
        max_pending: usize,
    ) -> Result<broadcast::Receiver<Vec<u8>>> {
        let receiver = self.subscribe(topic).await?;
        Ok(receiver.with_capacity(max_pending))
    }

    pub async fn publish_with_retry(
        &self,
        topic: &str,
        data: Vec<u8>,
        max_retries: usize,
        retry_delay: Duration,
    ) -> Result<()> {
        let mut attempts = 0;
        while attempts < max_retries {
            match self.publish(topic, data.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts == max_retries {
                        return Err(e);
                    }
                    sleep(retry_delay).await;
                }
            }
        }
        Ok(())
    }

    pub fn generate_subscription_resolver(
        &self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        topic: &str,
    ) -> Result<inkwell::values::FunctionValue<'ctx>> {
        let fn_type = self.context.void_type().fn_type(
            &[
                codegen.get_type("Context")?.into(),
                codegen.string_type().into(), // Subscription parameters
                codegen.get_type("SubscriptionCallback")?.into(),
            ],
            false,
        );

        let function = codegen.module.add_function(
            &format!("resolve_subscription_{}", topic),
            fn_type,
            None,
        );

        // Generate resolver implementation
        let builder = codegen.context.create_builder();
        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        self.generate_subscription_handler(&builder, function, topic)?;

        Ok(function)
    }
}
