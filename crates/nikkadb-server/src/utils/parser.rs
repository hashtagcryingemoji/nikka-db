use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;

pub fn parse(nikka_config_path: &str) -> Option<HashMap<String, String>> {
    let mut config = OpenOptions::new()
        .read(true)
        .open(nikka_config_path)
        .unwrap_or_else(|_| panic!("cannot find file {nikka_config_path}"));
    let mut content = String::new();

    config
        .read_to_string(&mut content)
        .unwrap_or_else(|_| panic!("cannot access {nikka_config_path}"));

    let x: Vec<String> = content
        .split("\r\n")
        .filter(|x| !x.is_empty())
        .filter(|x| !x.starts_with('#'))
        .map(ToString::to_string)
        .collect();
    let mut hm = HashMap::with_capacity(x.len());

    for line in x {
        let raw_line = line.split('#').next().unwrap();
        let content: Vec<String> = raw_line
            .split('=')
            .map(|x| x.trim())
            .map(ToString::to_string)
            .collect();

        assert_eq!(content.len(), 2, "invalid string: {line}");

        hm.insert(content[0].clone(), content[1].clone());
    }

    Some(hm)
}
