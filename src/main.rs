use clap::Parser;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    file: String,
}

#[derive(Debug, Clone)]
struct ExtResource {
    path: String,
    _type: String,
    id: String,
    uid: String,
}

impl ExtResource {
    fn new(path: String, _type: String, uid: String, id: String) -> Self {
        Self {
            path: path,
            _type: _type,
            id: id,
            uid: uid,
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
    instance: Option<ExtResource>,
    parameters: Vec<NodeParameter>,
    children: IndexMap<String, Node>,
    connections: Vec<Connection>,
}

impl Node {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            _type: "".to_string(),
            parent: "".to_string(),
            index: -1,
            instance: None,
            parameters: Vec::new(),
            children: IndexMap::new(),
            connections: Vec::new(),
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

#[derive(Debug, Clone)]
struct Connection {
    signal: String,
    from: String,
    to: String,
    method: String,
}

impl Connection {
    fn new(signal: &str, from: &str, to: &str, method: &str) -> Self {
        Self {
            signal: signal.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            method: method.to_string(),
        }
    }
}

/// Traverse and print the node tree
fn walk(node: &Node, prefix: &str) -> io::Result<()> {
    let mut index = node.children.len();
    if let Some(res) = &node.instance {
        if &node.parent != "" {
            if index == 0 {
                println!("{}    * ({}) {}", prefix, res._type, res.path);
            } else {
                println!("{}│   * ({}) {}", prefix, res._type, res.path);
            }
        }
    }
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
    for conn in node.connections.iter() {
        if index == 0 {
            println!("{}    * connection: {}:{}() => {}:{}()", prefix, conn.from, conn.signal, conn.to, conn.method);
        } else {
            println!("{}│   * connection: {}:{}() => {}:{}()", prefix, conn.from, conn.signal, conn.to, conn.method);
        }
    }
    for (name, child) in node.children.iter() {
        index -= 1;
        let node_type = match child.name == child._type || child._type == "".to_string() {
            true => "".to_string(),
            false => format!(" ({})", child._type),
        };
        if index == 0 {
            println!("{}└── {}{}", prefix, name, node_type);
            walk(&child, &format!("{}    ", prefix))?;
        } else {
            println!("{}├── {}{}", prefix, name, node_type);
            walk(&child, &format!("{}│   ", prefix))?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let f = File::open(&cli.file)?;
    let reader = BufReader::new(f);

    let ext_res_re = Regex::new(r#"^\[ext_resource (?P<remainder>.*)\]$"#).unwrap();
    let fields_re = Regex::new(r#"" "#).unwrap();
    let kv_re = Regex::new(r#"=""#).unwrap();
    let ext_res_id_re = Regex::new(r#".*ExtResource\([ "]*(?P<id>[0-9a-z_]+)[ "]*\)"#).unwrap();
    let sub_res_re = Regex::new(r#"^\[sub_resource type="(?P<type>[^"]+)" id="(?P<id>[0-9a-z_]+)".*\]$"#).unwrap();
    let sub_res_id_re = Regex::new(r#".*SubResource\([ "]*(?P<id>[0-9a-z_]+)[ "]*\)"#).unwrap();

    let node_re = Regex::new(r#"^\[node name="(?P<name>[^"]+)"(?P<remainder>.*)\]$"#).unwrap();
    let node_type_re = Regex::new(r#"type="(?P<type>[^"]+)".*"#).unwrap();
    let node_parent_re = Regex::new(r#"parent="(?P<parent>[^"]+)".*"#).unwrap();
    let node_index_re = Regex::new(r#"index="(?P<index>[^"]+)".*"#).unwrap();
    let node_instance_re = Regex::new(r#"instance=ExtResource\([ "]*(?P<instance>[0-9]+)[ "]*\).*"#).unwrap();

    let parameter_re = Regex::new(r"^(?P<k>[a-z][a-z_]*) = (?P<v>.*)").unwrap();
    let connection_re = Regex::new(r#"^\[connection signal="(?P<signal>[^"]+)" from="(?P<from>[^"]+)" to="(?P<to>[^"]+)" method="(?P<method>[^"]+)"\]"#).unwrap();

    let mut ext_resources = HashMap::new();
    let mut sub_resources = HashMap::new();
    let mut connections = Vec::<Connection>::new();
    let mut nodes = Vec::<Node>::new();
    let mut root = Node::new("");
    let mut last_sub_id = "".to_string();

    // parse scene file into structures
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = ext_res_re.captures(&line) {
            // line matches [ext_resource ...]
            let mut ext_res = ExtResource::new("".to_string(), "".to_string(), "".to_string(), "".to_string());
            let fields: Vec<&str> = fields_re.split(caps.name("remainder").unwrap().as_str()).collect();
            for field in fields {
                let kv: Vec<&str> = kv_re.split(field).collect();
                if kv[0] == "path" {
                    ext_res.path = kv[1].to_string();
                } else if kv[0] == "type" {
                    ext_res._type = kv[1].to_string();
                } else if kv[0] == "uid" {
                    ext_res.uid = kv[1].to_string();
                } else if kv[0] == "id" {
                    ext_res.id = kv[1].to_string().replace("\"", "");
                }
            }
            ext_resources.insert(ext_res.id.clone(), ext_res.clone());
        }
        else if let Some(caps) = sub_res_re.captures(&line) {
            // line matches [sub_resource ...]
            let id = match caps.name("id") {
                Some(id) => id.as_str().to_string(),
                None => "".to_string(),
            };
            last_sub_id = id.clone();
            sub_resources.insert(id, SubResource::new(String::from(caps.name("type").unwrap().as_str())));
        }
        else if let Some(caps) = node_re.captures(&line) {
            // line matches [node ...]
            let mut node = Node::new(caps.name("name").unwrap().as_str());
            if let Some(m) = caps.name("remainder") {
                let remainder = m.as_str();
                if let Some(c) = node_type_re.captures(remainder) {
                    if let Some(m) = c.name("type") {
                        node._type = m.as_str().to_string();
                    }
                }
                if let Some(c) = node_parent_re.captures(remainder) {
                    if let Some(m) = c.name("parent") {
                        node.parent = m.as_str().to_string();
                    }
                }
                if let Some(c) = node_index_re.captures(remainder) {
                    if let Some(m) = c.name("index") {
                        if let Ok(idx) = m.as_str().parse() {
                            node.index = idx;
                        }
                    }
                }
                if let Some(c) = node_instance_re.captures(remainder) {
                    if let Some(instance) = c.name("instance") {
                        node.instance = Some(ext_resources[instance.as_str()].clone());
                    }
                }
            }
            nodes.push(node);
        }
        else if let Some(caps) = parameter_re.captures(&line) {
            // line matches ___ = ___
            if nodes.len() == 0 {
                // no nodes parsed yet, these key/value pairs belong to sub resources
                if let Some(last_sub) = sub_resources.get_mut(&last_sub_id) {
                    (*last_sub).parameters.push(Parameter{
                        key: String::from(caps.name("k").unwrap().as_str()),
                        val: String::from(caps.name("v").unwrap().as_str()),
                    });
                }
            } else {
                // these key/value pairs belong to nodes
                if let Some(last_node) = nodes.last_mut() {
                    let val = String::from(caps.name("v").unwrap().as_str());
                    (*last_node).parameters.push(NodeParameter{
                        key: String::from(caps.name("k").unwrap().as_str()),
                        val: if val.starts_with("ExtResource") {
                                match ext_res_id_re.captures(&val) {
                                    Some(caps) => {
                                        match caps.name("id") {
                                            Some(id) => match ext_resources.get(id.as_str()) {
                                                Some(res) => format!("({}) {}", res._type.clone(), res.path.clone()),
                                                None => "".to_string(),
                                            },
                                            None => "".to_string(),
                                        }
                                    },
                                    None => "".to_string()
                                }
                            } else if val.starts_with("SubResource") {
                                match sub_res_id_re.captures(&val) {
                                    Some(caps) => {
                                        match caps.name("id") {
                                            Some(id) => {
                                                match sub_resources.get(id.as_str()) {
                                                    Some(res) => format!("({})", res._type.clone()),
                                                    None => "".to_string(),
                                                }
                                            },
                                            None => "".to_string(),
                                        }
                                    },
                                    None => {
                                        "".to_string()
                                    }
                                }
                            } else {
                                val.clone()
                            },
                        sub_params: if val.starts_with("SubResource") {
                                match sub_res_id_re.captures(&val) {
                                    Some(caps) => {
                                        match caps.name("id") {
                                            Some(id) => {
                                                let idx = id.as_str();
                                                match sub_resources.get(idx) {
                                                    Some(res) => res.parameters.clone(),
                                                    None => Vec::new(),
                                                }
                                            },
                                            None => Vec::new(),
                                        }
                                    },
                                    None => {
                                        Vec::new()
                                    }
                                }
                            } else {
                                Vec::new()
                            },
                    });
                }
            }
        }
        else if let Some(caps) = connection_re.captures(&line) {
            // line matches [connection ...]
            let conn = Connection::new(
                caps.name("signal").unwrap().as_str(),
                caps.name("from").unwrap().as_str(),
                match caps.name("to").unwrap().as_str() {
                    "." => nodes[0].name.as_str(),
                    s => s,
                },
                caps.name("method").unwrap().as_str(),
            );
            connections.push(conn);
        }
    }

    for mut node in nodes {
        // add connections to their corresponding source node
        node.connections = connections.iter().filter(|c| c.from == node.name || (c.from == ".".to_string() && node.parent == "".to_string())).cloned().collect();
        if node.parent == "".to_string() {
            // root node
            root = node;
        } else {
            // determine this node's parents and add it somewhere under the root node
            let parents: Vec<String>;
            if node.parent == ".".to_string() {
                parents = Vec::new();
            } else {
                parents = node.parent.split("/").map(|x| x.to_string()).collect();
            }
            root.add_child(node, parents)
        }
    }

    // print the scene tree to stdout
    print!("{}", root.name);
    if let Some(ext_res) = root.instance.clone() {
        println!(" ({}) {}", ext_res._type, ext_res.path);
    } else {
        println!("");
    }
    walk(&root, "")?;

    Ok(())
}
