use clap::Parser;
use indexmap::IndexMap;
use regex::Regex;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    file: String,
}

#[derive(Debug)]
struct ExtResource {
    path: String,
    _type: String,
}

impl ExtResource {
    fn new(path: String, _type: String) -> Self {
        Self {
            path: path,
            _type: _type,
        }
    }
}

#[derive(Debug)]
struct SubResource {
    _type: String,
    parameters: Vec<Parameter>,
}

impl SubResource {
    fn new(_type: String) -> Self {
        Self {
            _type: _type,
            parameters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Parameter {
    key: String,
    val: String,
}

#[derive(Debug, Clone)]
struct NodeParameter {
    key: String,
    val: String,
    sub_params: Vec<Parameter>,
}

#[derive(Debug, Clone)]
struct Node {
    name: String,
    _type: String,
    parent: String,
    index: i32,
    instance: usize,
    parameters: Vec<NodeParameter>,
    children: IndexMap<String, Node>,
}

impl Node {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            _type: "".to_string(),
            parent: "".to_string(),
            index: -1,
            instance: 0,
            parameters: Vec::new(),
            children: IndexMap::new(),
        }
    }
    fn add_child(&mut self, node: Node, mut parents: Vec<String>) {
        if parents.len() > 0 {
            let parent = parents.remove(0);
            let child = self.children.entry(parent).or_insert(Node::new(""));
            child.add_child(node, parents);
        } else {
            self.children.entry(node.name.clone()).or_insert(node);
        }
    }
}

fn walk(node: &Node, prefix: &str) -> io::Result<()> {
    let mut index = node.children.len();
    for param in node.parameters.iter() {
        if index == 0 {
            println!("{}    * {}: {}", prefix, param.key, param.val);
        } else {
            println!("{}│   * {}: {}", prefix, param.key, param.val);
        }
        let mut sub_index = param.sub_params.len();
        let padding = (0..param.key.chars().count()+2).map(|_| " ").collect::<String>();
        for sub in param.sub_params.iter() {
            sub_index -= 1;
            if sub_index == 0 {
                println!("{}      {}└── {}: {}", prefix, padding, sub.key, sub.val);
            } else {
                println!("{}      {}├── {}: {}", prefix, padding, sub.key, sub.val);
            }
        }
    }
    for (name, child) in node.children.iter() {
        index -= 1;
        if index == 0 {
            println!("{}└── {}", prefix, name);
            walk(&child, &format!("{}    ", prefix))?;
        } else {
            println!("{}├── {}", prefix, name);
            walk(&child, &format!("{}│   ", prefix))?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let f = File::open(&cli.file)?;
    let reader = BufReader::new(f);

    let ext_res_re = Regex::new(r#"^\[ext_resource path="(?P<path>[^"]+)" type="(?P<type>[^"]+)" id=(?P<id>[0-9]+).*\]$"#).unwrap();
    let sub_res_re = Regex::new(r#"^\[sub_resource type="(?P<type>[^"]+)" id=(?P<id>[0-9]+).*\]$"#).unwrap();

    let node_re = Regex::new(r#"^\[node name="(?P<name>[^"]+)"(?P<remainder>.*)\]$"#).unwrap();
    let node_type_re = Regex::new(r#"type="(?P<type>[^"]+)".*"#).unwrap();
    let node_parent_re = Regex::new(r#"parent="(?P<parent>[^"]+)".*"#).unwrap();
    let node_index_re = Regex::new(r#"index="(?P<index>[^"]+)".*"#).unwrap();
    let node_instance_re = Regex::new(r#"instance=ExtResource\( (?P<instance>[^"]+) \).*"#).unwrap();

    let parameter_re = Regex::new(r"^(?P<k>[a-z][a-z_]*) = (?P<v>.*)").unwrap();

    let mut ext_resources = vec![ExtResource::new("".to_string(), "".to_string())];
    let mut sub_resources = vec![SubResource::new("".to_string())];
    let mut nodes: Vec<Node> = Vec::new();
    let mut root = Node::new("");

    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = ext_res_re.captures(&line) {
            while ext_resources.len() < caps.name("id").unwrap().as_str().parse().unwrap() {
                ext_resources.push(ExtResource::new("".to_string(), "".to_string()));
            }
            ext_resources.push(ExtResource::new(
                String::from(caps.name("path").unwrap().as_str()),
                String::from(caps.name("type").unwrap().as_str()),
            ));
        }
        else if let Some(caps) = sub_res_re.captures(&line) {
            while sub_resources.len() < caps.name("id").unwrap().as_str().parse().unwrap() {
                sub_resources.push(SubResource::new("".to_string()));
            }
            sub_resources.push(SubResource::new(
                String::from(caps.name("type").unwrap().as_str()),
            ));
        }
        else if let Some(caps) = node_re.captures(&line) {
            let mut node = Node::new(caps.name("name").unwrap().as_str());
            if let Some(caps) = node_type_re.captures(caps.name("remainder").unwrap().as_str()) {
                node._type = String::from(caps.name("type").unwrap().as_str());
            }
            if let Some(caps) = node_parent_re.captures(caps.name("remainder").unwrap().as_str()) {
                node.parent = String::from(caps.name("parent").unwrap().as_str());
            }
            if let Some(caps) = node_index_re.captures(caps.name("remainder").unwrap().as_str()) {
                node.index = caps.name("index").unwrap().as_str().parse().unwrap();
            }
            if let Some(caps) = node_instance_re.captures(caps.name("remainder").unwrap().as_str()) {
                node.instance = caps.name("instance").unwrap().as_str().parse().unwrap();
            }
            nodes.push(node);
        }
        else if let Some(caps) = parameter_re.captures(&line) {
            if nodes.len() == 0 {
                if let Some(last_sub) = sub_resources.last_mut() {
                    (*last_sub).parameters.push(Parameter{
                        key: String::from(caps.name("k").unwrap().as_str()),
                        val: String::from(caps.name("v").unwrap().as_str()),
                    });
                }
            } else {
                if let Some(last_node) = nodes.last_mut() {
                    let val = String::from(caps.name("v").unwrap().as_str());
                    (*last_node).parameters.push(NodeParameter{
                        key: String::from(caps.name("k").unwrap().as_str()),
                        val: if val.starts_with("ExtResource") {
                                let idx: usize = val.replace("ExtResource( ", "").replace(" )", "").parse().unwrap();
                                ext_resources[idx].path.clone()
                                // format!("{:?}", ext_resources[idx])
                            } else if val.starts_with("SubResource") {
                                let idx: usize = val.replace("SubResource( ", "").replace(" )", "").parse().unwrap();
                                sub_resources[idx]._type.clone()
                            } else {
                                val.clone()
                            },
                        sub_params: if val.starts_with("SubResource") {
                                let idx: usize = val.replace("SubResource( ", "").replace(" )", "").parse().unwrap();
                                sub_resources[idx].parameters.clone()
                            } else {
                                Vec::new()
                            },
                    });
                }
            }
        }
    }

    for node in nodes {
        if node.parent == "".to_string() {
            // root node
            root = node;
        } else {
            let parents: Vec<String>;
            if node.parent == ".".to_string() {
                parents = Vec::new();
            } else {
                parents = node.parent.split("/").map(|x| x.to_string()).collect();
            }
            //let parent = parents.remove(0);
            root.add_child(node, parents)
            // nodes.push(node);
        }
    }

    println!("{}", root.name);
    walk(&root, "")?;

    Ok(())
}


/*

├
│
└

*/