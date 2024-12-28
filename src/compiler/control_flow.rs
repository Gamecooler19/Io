use std::collections::{HashMap, HashSet};
use crate::{
    ast::{ASTNode, ASTVisitor},
    error::IoError,
    Result,
};

#[derive(Debug)]
pub struct BasicBlock {
    id: usize,
    statements: Vec<ASTNode>,
    successors: Vec<usize>,
    predecessors: Vec<usize>,
    dominators: HashSet<usize>,
}

pub struct ControlFlowGraph {
    blocks: HashMap<usize, BasicBlock>,
    entry: usize,
    exit: usize,
    current_id: usize,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        let mut cfg = Self {
            blocks: HashMap::new(),
            entry: 0,
            exit: 1,
            current_id: 2,
        };

        // Create entry and exit blocks
        cfg.blocks.insert(0, BasicBlock {
            id: 0,
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            dominators: HashSet::new(),
        });

        cfg.blocks.insert(1, BasicBlock {
            id: 1,
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            dominators: HashSet::new(),
        });

        cfg
    }

    pub fn analyze_function(&mut self, body: &[ASTNode]) -> Result<()> {
        let mut analyzer = BlockAnalyzer::new(self);
        for node in body {
            analyzer.visit_node(node)?;
        }
        self.compute_dominators()?;
        Ok(())
    }

    fn compute_dominators(&mut self) -> Result<()> {
        // Initialize dominators
        let mut changed = true;
        let all_blocks: HashSet<_> = self.blocks.keys().cloned().collect();

        // Entry block only dominates itself initially
        if let Some(entry) = self.blocks.get_mut(&self.entry) {
            entry.dominators = HashSet::from([self.entry]);
        }

        // All other blocks are dominated by all blocks initially
        for (&id, block) in self.blocks.iter_mut() {
            if id != self.entry {
                block.dominators = all_blocks.clone();
            }
        }

        // Iteratively compute dominators
        while changed {
            changed = false;
            for id in 0..self.current_id {
                if id == self.entry {
                    continue;
                }

                if let Some(block) = self.blocks.get(&id) {
                    let mut new_doms = all_blocks.clone();
                    
                    // Intersect dominators of all predecessors
                    for &pred in &block.predecessors {
                        if let Some(pred_block) = self.blocks.get(&pred) {
                            new_doms = new_doms.intersection(&pred_block.dominators)
                                .cloned()
                                .collect();
                        }
                    }

                    // Add self to dominators
                    new_doms.insert(id);

                    // Check if dominators changed
                    if let Some(block) = self.blocks.get_mut(&id) {
                        if block.dominators != new_doms {
                            block.dominators = new_doms;
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        // Verify that all blocks are reachable from entry
        let mut visited = HashSet::new();
        let mut stack = vec![self.entry];

        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue;
            }

            if let Some(block) = self.blocks.get(&id) {
                stack.extend(&block.successors);
            }
        }

        // Check if any blocks are unreachable
        for &id in self.blocks.keys() {
            if !visited.contains(&id) {
                return Err(IoError::validation_error(
                    format!("Basic block {} is unreachable", id)
                ));
            }
        }

        Ok(())
    }
}

struct BlockAnalyzer<'a> {
    cfg: &'a mut ControlFlowGraph,
    current_block: usize,
}

impl<'a> BlockAnalyzer<'a> {
    fn new(cfg: &'a mut ControlFlowGraph) -> Self {
        Self {
            cfg,
            current_block: cfg.entry,
        }
    }

    fn create_block(&mut self) -> usize {
        let id = self.cfg.current_id;
        self.cfg.current_id += 1;
        self.cfg.blocks.insert(id, BasicBlock {
            id,
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            dominators: HashSet::new(),
        });
        id
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if let Some(block) = self.cfg.blocks.get_mut(&from) {
            if !block.successors.contains(&to) {
                block.successors.push(to);
            }
        }
        if let Some(block) = self.cfg.blocks.get_mut(&to) {
            if !block.predecessors.contains(&from) {
                block.predecessors.push(from);
            }
        }
    }
}
