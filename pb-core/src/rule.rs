pub struct Tool;

impl Tool {
    pub fn run(&self, args: &[&dyn IntoArg]) {}
}

struct ActionBuilder {}

impl ActionBuilder {
    pub fn tool(&self, name: String) -> Tool {
        Tool
    }

    pub fn create_file(&self, name: String) -> File {
        File
    }

    pub fn create_dir(&self, name: String) -> Dir {
        Dir
    }

    pub fn read_file(&self, file: &File) -> Vec<String> {
        vec![]
    }
}

pub trait IntoArg {
    fn into_arg(&self) -> String;
}

impl IntoArg for String {
    fn into_arg(&self) -> String {
        self.clone()
    }
}

struct File;

impl IntoArg for File {
    fn into_arg(&self) -> String {
        "blah".to_string()
    }
}

struct Dir;

trait Rule {
    type Args;
    type Tools;

    type Output;

    fn exec(args: Self::Args, tools: Self::Tools) -> Self::Output;
}

struct HttpArgs {
    url: String,
    hash: String,
}

struct HttpTools {}

struct HttpOutput {}

enum AllRules {
    Http { url: String, hash: String },
}

fn exec_rule(builder: &ActionBuilder, rule: AllRules) {
    match rule {
        AllRules::Http { url, hash } => {
            http_rule(builder, url, hash);
        }
    }
}

fn http_rule(
    builder: &ActionBuilder,
    url: String,
    hash: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = builder.create_file("foo".to_string());
    let tool = builder.tool("curl".to_string());

    let args: &[&dyn IntoArg] = &[&url, &output];
    tool.run(&args[..]);

    

    Ok(())
}
