use std::collections::HashMap;

use crate::handler::{BoxedHandler, RefHandler};

#[derive(Default)]
struct Node<'a> {
    childrens: Option<HashMap<&'a str, Node<'a>>>,
    wildcard_node: Option<Box<Node<'a>>>,
    value: Option<BoxedHandler>,
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

    pub fn extend(&mut self, another_handler: HandlerSelect<'a>) {
        let root = another_handler.root;

        self.rec_extend(root, "".to_owned());
    }

    fn rec_extend(&mut self, node: Node<'a>, mut path: String) {
        if node.value.is_some() {
            return self.insert(Box::leak(path.into_boxed_str()), node.value.unwrap());
        }

        match (node.childrens, node.wildcard_node) {
            (None, None) => unreachable!("Something went wrong, can't exist a Node that don't have either a Handler or children nodes"),
            (None, Some(wildcard_node)) => {
                path.push_str("/{wildcard_node}");
                self.rec_extend(*wildcard_node, path);
            }
            (Some(childrens), None) => {
                childrens.into_iter().for_each(|(next_path_segment, node)| {
                    self.rec_extend(node, format!("{}/{}", path, next_path_segment));
                });
            }
            (Some(childrens), Some(wildcard_node)) => {
                childrens.into_iter().for_each(|(next_path_segment, node)| {
                    self.rec_extend(node, format!("{}/{}", path, next_path_segment));
                });

                path.push_str("{wildcard_node}");
                self.rec_extend(*wildcard_node, path);
            }
        };
    }

    pub fn insert(&mut self, path: &'a str, handler: BoxedHandler) {
        let mut node = &mut self.root;
        for splitted_path in path.split('/').filter(|x| !x.is_empty()) {
            if is_parameter_declaration(splitted_path) {
                node = node.add_wildcard_node();
                continue;
            }

            node = node.add_normal_node(splitted_path);
        }

        if node.value.is_some() {
            panic!("{} already defined", path);
        }
        node.value = Some(handler);
    }

    pub fn get(&self, path: &str) -> Option<RefHandler<'_>> {
        let mut root = &self.root;

        for splitted_path in path.split('/').filter(|x| !x.is_empty()) {
            match (&root.childrens, &root.wildcard_node) {
                (None, None) => return None,
                (None, Some(wildcard_node)) => root = wildcard_node.as_ref(),
                (Some(childrens), None) => {
                    if childrens.contains_key(splitted_path) {
                        root = childrens.get(splitted_path).unwrap();
                    } else {
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
