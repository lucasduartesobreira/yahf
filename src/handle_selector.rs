use std::collections::HashMap;

use crate::handler::{BoxedAsyncHandler, RefAsyncHandler};

#[derive(Default)]
struct Node<'a> {
    childrens: Option<HashMap<&'a str, Node<'a>>>,
    wildcard_node: Option<Box<Node<'a>>>,
    value: Option<BoxedAsyncHandler>,
}

#[derive(Default)]
pub struct HandlerSelect<'a> {
    root: Node<'a>,
}

fn is_parameter_declaration(value: &str) -> bool {
    value.starts_with('{') && value.ends_with('}')
}

impl<'a> HandlerSelect<'a> {
    pub fn new() -> Self {
        Self {
            root: Node::default(),
        }
    }

    pub fn insert(&mut self, path: &'a str, handler: BoxedAsyncHandler) {
        let mut node = &mut self.root;
        for splitted_path in path.split('/').filter(|x| !x.is_empty()) {
            if is_parameter_declaration(splitted_path) {
                node = node.add_wildcard_node();
                continue;
            }

            node = node.add_normal_node(splitted_path);
        }

        node.value = Some(handler);
    }

    #[allow(dead_code)]
    pub fn get(&self, path: &'a str) -> Option<RefAsyncHandler<'_>> {
        let mut root = &self.root;

        for splitted_path in path.split('/').filter(|x| !x.is_empty()) {
            match (&root.childrens, &root.wildcard_node) {
                (None, None) => return None,
                (None, Some(wildcard_node)) => root = wildcard_node.as_ref(),
                (Some(childrens), None) => {
                    if childrens.contains_key(splitted_path) {
                        println!("Sla");
                        root = childrens.get(splitted_path).unwrap();
                    } else {
                        println!("Sla2");
                        return None;
                    }
                }
                (Some(childrens), Some(wildcard_node)) => {
                    if childrens.contains_key(splitted_path) {
                        root = childrens.get(splitted_path).unwrap();
                        continue;
                    }

                    root = wildcard_node.as_ref();
                }
            }
        }

        root.value.as_ref().map(|boxed| boxed.as_ref())
    }
}

impl<'a> Node<'a> {
    fn add_wildcard_node(&mut self) -> &mut Self {
        if self.wildcard_node.is_some() {
            return self.wildcard_node.as_mut().unwrap().as_mut();
        }

        self.wildcard_node = Some(Box::default());
        self.wildcard_node.as_mut().unwrap().as_mut()
    }

    fn add_normal_node(&mut self, path: &'a str) -> &mut Self {
        if self.childrens.is_none() {
            self.childrens = Some(HashMap::new());
        }

        match self.childrens.as_mut() {
            Some(childrens) => childrens.entry(path).or_insert(Node::default()),
            None => {
                unreachable!("LALALALALA")
            }
        }
    }
}
