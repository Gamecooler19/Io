pub use crate::ast::{Expression, Function, Module, Parameter, Statement};
pub use crate::error::Result;

// Inkwell re-exports
pub use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    debug_info::*,
    module::Module as LLVMModule,
    passes::PassManager,
    targets::{CodeModel, FileType, RelocMode, Target, TargetMachine},
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, IntType, PointerType, VoidType},
    values::{BasicValue, BasicValueEnum, CallSiteValue, FunctionValue, PointerValue},
    AddressSpace,
};

// Standard library re-exports
pub use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context as TaskContext, Poll, Wake, Waker},
};

// External crate re-exports
pub use futures::task::noop_waker;
pub use tokio::sync::oneshot;
