use async_std::io::{Error, Write, Read};
use futures::AsyncReadExt;
use std::future::Future;
use std::io::ErrorKind;
use std::path::Path;
use std::process::ExitStatus;
use std::pin::Pin;
use async_std::process::{Child, Command, Output, Stdio, ChildStdout, ChildStdin};
use std::sync::{Arc, Mutex};
use log::{debug, info, trace, warn};
use crate::util::temp_file::TempFile;

#[derive(Debug)]
pub struct IsolateSandbox {
    box_id: u32,
    box_path: String
}

async fn panic_on_fail(command: &mut Command) -> Output {
    let output = command.output().await;

    if let Ok(output) = output {
        if output.status.success() {
            return output;
        } else {
            println!("Standard output:");
            println!("{}", String::from_utf8_lossy(&output.stdout));

            println!("Standard error:");
            println!("{}", String::from_utf8_lossy(&output.stderr));

            panic!("Command failed!");
        }
    } else {
        panic!("Command failed!");
    }
}

fn actual_path(path: &str) -> String {
    #[cfg(target_os = "windows")] {
        let mut absolute_path = path.to_string();

        //Check if second character is not a colon
        if absolute_path.chars().nth(1).unwrap() != ':' {
            //Check if first character is a slash
            if absolute_path.chars().nth(0).unwrap() == '/' {
                //Remove first character
                absolute_path = absolute_path[1..].to_string();
            }

            //Add current working directory
            absolute_path = format!("{}/{}", std::env::current_dir().unwrap().to_str().unwrap(), absolute_path);
        }

        let mut resolved_path = absolute_path.replace("\\", "/").replace(":", "");
        //Make first character lowercase
        resolved_path.replace_range(0..1, &resolved_path[0..1].to_lowercase());
        format!("/mnt/{}", resolved_path)
    }

    #[cfg(not(target_os = "windows"))] {
        // Get absolute path
        let p = Path::new(path);
        let canon = match p.canonicalize() {
            Ok(canon) => canon,
            Err(e) => panic!("Failed to canonicalize path '{}': {}", path, e)
        };
        let absolute_path = canon.to_str().unwrap();

        absolute_path.to_string()
    }
}

/*
pub fn launch(
        &self,
        program: String, args: Vec<String>, 
        mapped_dirs: Vec<(String, String)>,
        env_vars: Vec<(String, String)>,
        stdin_file: Option<String>, options: &LaunchOptions
    ) -> RunningJob {
*/

#[derive(Debug, Clone, Copy)]
pub enum DirectoryOption {
    ReadWrite,
    Dev,
    NoExec,
    Maybe,
    Filesystem,
    NoRecursive
}

impl DirectoryOption {
    fn key(&self) -> &'static str {
        match self {
            DirectoryOption::ReadWrite => "rw",
            DirectoryOption::Dev => "dev",
            DirectoryOption::NoExec =>"noexec",
            DirectoryOption::Maybe => "maybe",
            DirectoryOption::Filesystem => "fs",
            DirectoryOption::NoRecursive => "norec",
        }
    }
}

#[derive(Debug, Clone)]
pub enum MappingKind {
    NamedMapping(String, String),
    FullMapping(String)
}

#[derive(Debug, Clone)]
pub struct DirMapping {
    mapping: MappingKind,
    options: Vec<DirectoryOption>
}

impl DirMapping {
    pub fn named<A: Into<String>, B: Into<String>>(sandbox_path: A, external_path: B) -> Self {
        Self {
            mapping: MappingKind::NamedMapping(sandbox_path.into(), external_path.into()),

            options: vec![]
        }
    }

    pub fn full<A: Into<String>>(path: A) -> Self {
        Self {
            mapping: MappingKind::FullMapping(path.into()),

            options: vec![]
        }
    }

    pub fn read_write(mut self) -> Self {
        self.options.push(DirectoryOption::ReadWrite);
        self
    }

    pub fn dev(mut self) -> Self {
        self.options.push(DirectoryOption::Dev);
        self
    }

    pub fn no_exec(mut self) -> Self {
        self.options.push(DirectoryOption::NoExec);
        self
    }

    pub fn maybe(mut self) -> Self {
        self.options.push(DirectoryOption::Maybe);
        self
    }

    pub fn filesystem(mut self) -> Self {
        self.options.push(DirectoryOption::Filesystem);
        self
    }

    pub fn no_recursive(mut self) -> Self {
        self.options.push(DirectoryOption::NoRecursive);
        self
    }

    pub fn to_arg(&self) -> String {
        let mut res = match &self.mapping {
            MappingKind::NamedMapping(sandbox_path, external_path) => format!("--dir={}={}", sandbox_path, actual_path(&external_path)),
            MappingKind::FullMapping(path) => format!("--dir={}", actual_path(&path))
        };

        for opt in &self.options {
            res.push_str(&format!(":{}", opt.key()));
        }

        res
    }
}

#[derive(Debug, Clone)]
pub enum EnvRule {
    Inherit(String),
    SetValue(String, String),
    InheritAll
}

impl EnvRule {
    pub fn to_arg(&self) -> String {
        match self {
            EnvRule::Inherit(var) => format!("--env={var}"),
            EnvRule::SetValue(var, value) => format!("--env={var}={value}"),
            EnvRule::InheritAll => "--full-env".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct LaunchOptions {
    pub memory_limit_kb: Option<u32>,
    pub time_limit_s: Option<f32>,
    pub wall_time_limit_s: Option<f32>,
    pub extra_time_s: Option<f32>,

    pub max_process: MaxProcessCount,
    pub mapped_dirs: Vec<DirMapping>,
    pub env: Vec<EnvRule>
}

impl LaunchOptions {
    pub fn new() -> Self {
        Self {
            memory_limit_kb: None,
            time_limit_s: None,
            wall_time_limit_s: None,
            extra_time_s: None,

            max_process: MaxProcessCount::Fixed(1),
            mapped_dirs: vec![],
            env: vec![]
        }
    }

    pub fn get_memory_limit_kb(&self) -> u32 {
        self.memory_limit_kb.unwrap_or(4 * 1024 * 1024)
    }

    pub fn get_time_limit_s(&self) -> f32 {
        self.time_limit_s.unwrap_or(1.0)
    }

    pub fn get_wall_time_limit_s(&self) -> f32 {
        self.wall_time_limit_s.unwrap_or(3f32 * self.get_time_limit_s() + self.get_extra_time_s() + 5f32)
    }

    pub fn get_extra_time_s(&self) -> f32 {
        self.extra_time_s.unwrap_or(0.5)
    }

    pub fn memory_limit_kb(mut self, memory_limit_kb: u32) -> Self {
        self.memory_limit_kb = Some(memory_limit_kb);
        self
    }

    pub fn time_limit_s(mut self, time_limit_s: f32) -> Self {
        self.time_limit_s = Some(time_limit_s);
        self
    }

    pub fn wall_time_limit_s(mut self, wall_time_limit_s: f32) -> Self {
        self.wall_time_limit_s = Some(wall_time_limit_s);
        self
    }

    pub fn extra_time_s(mut self, extra_time_s: f32) -> Self {
        self.extra_time_s = Some(extra_time_s);
        self
    }

    pub fn max_processes(mut self, max_process: MaxProcessCount) -> Self {
        self.max_process = max_process;
        self
    }

    pub fn add_mapping(mut self, mapping: DirMapping) -> Self {
        self.mapped_dirs.push(mapping);
        self
    }

    pub fn map_dir<A: Into<String>, B: Into<String>>(self, internal: A, external: B) -> Self {
        self.add_mapping(DirMapping::named(internal, external))
    }

    pub fn map_full<A: Into<String>>(self, path: A) -> Self {
        self.add_mapping(DirMapping::full(path))
    }

    pub fn env_rule(mut self, env: EnvRule) -> Self {
        self.env.push(env);
        self
    }

    pub fn inherit<A: Into<String>>(self, var: A) -> Self {
        self.env_rule(EnvRule::Inherit(var.into()))
    }

    pub fn set_env<A: Into<String>, B: Into<String>>(self, var: A, value: B) -> Self {
        self.env_rule(EnvRule::SetValue(var.into(), value.into()))
    }

    pub fn full_env(self) -> Self {
        self.env_rule(EnvRule::InheritAll)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MaxProcessCount {
    Fixed(usize),
    Unlimited
}

pub struct LaunchInfo {
    pub child: Child,
    pub metafile_path: String,
}

pub struct RunningJob {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<ChildStdout>>,

    _metafile: TempFile,
    pub stderr: TempFile,

    killed: bool,
    attempt_kill: bool,

    error_message: Option<String>,
    on_exit: Option<Box<dyn FnOnce(&mut RunningJob) + Sync + Send>>,
}

pub struct WriteFuture<'data> {
    stdin: Arc<Mutex<ChildStdin>>,
    data: &'data [u8],
    pos: usize,
}

impl<'data> WriteFuture<'data> {
    pub fn new(stdin: Arc<Mutex<ChildStdin>>, data: &'data [u8]) -> WriteFuture {
        WriteFuture {
            stdin,
            data,
            pos: 0,
        }
    }
}

impl<'data> Future for WriteFuture<'data> {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let res = {
            let mut stdin = self.stdin.lock().unwrap();
            let pinned = Pin::new(&mut *stdin);
            pinned.poll_write(cx, &self.data[self.pos..])
        };

        match res {
            std::task::Poll::Ready(Ok(0)) => {
                std::task::Poll::Ready(Err(Error::new(ErrorKind::WriteZero, "Failed to write data")))
            }
            std::task::Poll::Ready(Ok(bytes)) => {
                self.pos += bytes;
                if self.pos == self.data.len() {
                    std::task::Poll::Ready(Ok(()))
                } else {
                    std::task::Poll::Pending
                }
            },
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

pub struct ReadFuture<'data> {
    stdout: Arc<Mutex<ChildStdout>>,
    data: &'data mut [u8],
    pos: usize,
    remaining: usize,
}

impl<'data> ReadFuture<'data> {
    pub fn new(stdout: Arc<Mutex<ChildStdout>>, data: &'data mut [u8]) -> ReadFuture {
        Self::new_with_size(stdout, data, data.len())
    }

    pub fn new_with_size(stdout: Arc<Mutex<ChildStdout>>, data: &'data mut [u8], size: usize) -> ReadFuture {
        if size > data.len() {
            panic!("Size is larger than buffer size");
        }

        ReadFuture {
            stdout,
            data,
            pos: 0,
            remaining: size,
        }
    }
}

impl<'data> Future for ReadFuture<'data> {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let res = {
            let ReadFuture {ref stdout, ref mut data, ref pos, ref remaining} = &mut *self;
            let mut stdout = stdout.lock().unwrap();
            let pinned = Pin::new(&mut *stdout);
            pinned.poll_read(cx, &mut data[*pos..*pos + *remaining])
        };

        match res {
            std::task::Poll::Ready(Ok(0)) => std::task::Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF"))),
            std::task::Poll::Ready(Ok(bytes)) => {
                self.pos += bytes;
                self.remaining -= bytes;
                if self.remaining == 0 {
                    std::task::Poll::Ready(Ok(()))
                } else {
                    std::task::Poll::Pending
                }
            },
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

macro_rules! read_impl {
    ($name:ident, $ty:ty, $size:expr) => {
        pub async fn $name(&mut self) -> Result<$ty, Error> {
            let mut buf = [0u8; $size];
            self.read(&mut buf).await?;
            Ok(<$ty>::from_le_bytes(buf))
        }
    };
}

impl RunningJob {
    pub fn new(mut child: Child, stderr: TempFile, metafile: TempFile, on_exit: Option<Box<dyn FnOnce(&mut RunningJob) + Sync + Send>>) -> RunningJob {
        let stdin = Arc::new(Mutex::new(child.stdin.take().unwrap()));
        let stdout = Arc::new(Mutex::new(child.stdout.take().unwrap()));

        RunningJob {
            child,
            stdin,
            stdout,
            _metafile: metafile,
            stderr,

            killed: false,
            attempt_kill: false,
            
            error_message: None,

            on_exit,
        }
    }

    fn set_on_exit<T: FnOnce(&mut RunningJob) + Sync + Send + 'static>(&mut self, on_exit: T) {
        self.on_exit = Some(Box::new(on_exit));
    }

    pub fn add_pre_exit<T: FnOnce(&mut RunningJob) + Sync + Send + 'static>(&mut self, on_exit: T) {
        match self.on_exit.take() {
            Some(post) => {
                self.set_on_exit(move |job| {
                    on_exit(job);
                    post(job);
                })
            },
            None => self.set_on_exit(on_exit)
        }
    }

    pub fn add_post_exit<T: FnOnce(&mut RunningJob) + Sync + Send + 'static>(&mut self, on_exit: T) {
        match self.on_exit.take() {
            Some(pre) => {
                self.set_on_exit(move |job| {
                    pre(job);
                    on_exit(job);
                })
            },
            None => self.set_on_exit(on_exit)
        }
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn get_error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        WriteFuture::new(self.stdin.clone(), data).await
    }

    pub async fn read(&mut self, data: &mut [u8]) -> Result<(), std::io::Error> {
        ReadFuture::new(self.stdout.clone(), data).await
    }

    read_impl!(read_u8, u8, 1);
    read_impl!(read_u16, u16, 2);
    read_impl!(read_u32, u32, 4);
    read_impl!(read_u64, u64, 8);

    read_impl!(read_i8, i8, 1);
    read_impl!(read_i16, i16, 2);
    read_impl!(read_i32, i32, 4);
    read_impl!(read_i64, i64, 8);

    read_impl!(read_f32, f32, 4);
    read_impl!(read_f64, f64, 8);

    pub async fn read_str(&mut self) -> Result<String, Error> {
        let len = self.read_u32().await? as usize;
        let mut buf = vec![0u8; len];
        self.read(&mut buf).await?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    pub async fn read_bool(&mut self) -> Result<bool, Error> {
        let val = self.read_u8().await?;
        Ok(val != 0)
    }

    pub async fn wait(&mut self) -> Result<ExitStatus, Error> {
        match self.child.status().await {
            Ok(x) => {
                if !x.success() {
                    println!("Child exited with status {}", x);
                }

                Ok(x)
            },
            Err(e) => Err(e)
        }
    }

    pub async fn kill(&mut self) -> Result<(), Error> {
        if self.killed {
            return Ok(());
        }

        self.attempt_kill = true;

        let pid = self.child.id();

        let mut command = Command::new("pgrep");
        command.arg("-P");
        command.arg(pid.to_string());
        command.stderr(Stdio::null());
        command.stdout(Stdio::piped());

        let output = command.output().await?;

        let children = String::from_utf8_lossy(&output.stdout);

        for child in children.lines() {
            let child = child.trim();
            if child.len() == 0 {
                continue;
            }

            let child = child.parse::<u32>().unwrap();
            let mut command = Command::new("kill");
            command.arg("-9");
            command.arg(child.to_string());

            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
            command.stdin(Stdio::null());

            command.spawn().unwrap();
        }

        self.child.kill()?;

        self.killed = true;

        if let Some(on_exit) = self.on_exit.take() {
            on_exit(self);
        }

        Ok(())
    }

    pub async fn read_stderr(&self, max: Option<usize>) -> String {
        let mut file = self.stderr.get_file_async_read().await;

        if let Some(max) = max {
            let mut buf = vec![0u8; max];
            let mut num_read = 0; 
            loop {
                let res = file.read(&mut buf[num_read..]).await.unwrap();

                if res == 0 {
                    break;
                }

                num_read += res;
            }
            let mut res = String::from_utf8_lossy(&buf[..num_read]).to_string();

            if num_read == max {
                //Remove U+FFFD REPLACEMENT CHARACTER if it is last
                if res.ends_with("\u{FFFD}") {
                    res.pop();
                }

                res.push_str("...\n");
                res.push_str(&format!("(stderr truncated to {} bytes)", max));
                res
            } else {
                res
            }
        } else {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.unwrap();
            String::from_utf8_lossy(&buf).to_string()
        }
    }
}

//Ensure that the child process is killed when the RunningJob is dropped
impl Drop for RunningJob {
    fn drop(&mut self) {
        if !self.killed {
            warn!("RunningJob dropped without being explicitly killed");
            async_std::task::block_on(self.kill()).unwrap();
        }
    }
}

const ISOLATE_PATH: &str = "isolate";

pub async fn make_public(dir: &str) {
    let mut command = Command::new("chmod");
    command.arg("-R");
    command.arg("a+rw");
    command.arg(dir);

    command.output().await.unwrap();
}

impl IsolateSandbox {
    pub async fn new(id: u32) -> IsolateSandbox {
        let mut sandbox = IsolateSandbox {
            box_id: id,
            box_path: "".to_string()
        };

        sandbox.cleanup().await;
        sandbox.initialize().await;

        sandbox
    }

    async fn initialize(&mut self) {
        info!("Initializing sandbox {}", self.box_id);
        let mut command = Command::new(ISOLATE_PATH);

        command.arg("--init");
        command.arg("--box-id");
        command.arg(self.box_id.to_string());

        let output = panic_on_fail(&mut command).await;

        let output = String::from_utf8_lossy(&output.stdout);

        let lines: Vec<&str> = output.split("\n").collect();
        let box_path = lines[0].to_string();
        debug!("Box path: {}", box_path);

        self.box_path = box_path;
    }

    async fn cleanup(&self) {
        info!("Cleaning up sandbox {}", self.box_id);
        let mut command = Command::new(ISOLATE_PATH);

        command.arg("--cleanup");
        command.arg("--box-id");
        command.arg(self.box_id.to_string());

        panic_on_fail(&mut command).await;
    }

    pub fn launch(
        &self,
        program: String, args: Vec<String>, 
        options: &LaunchOptions
    ) -> RunningJob {
        trace!("Launching command {} in sandbox {}", program, self.box_id);
        let mut command = Command::new(ISOLATE_PATH);

        let metafile_file = TempFile::with_extra(".meta");
        let stderr_file = TempFile::with_extra(".stderr");
        

        command.args(vec![
            "--box-id",
            &self.box_id.to_string(),
            format!("--meta={}", actual_path(&metafile_file.path)).as_str(),
        ]);

        for mapping in &options.mapped_dirs {
            command.arg(mapping.to_arg());
        }

        for env in &options.env {
            command.arg(env.to_arg());
        }

        command.args(vec![
            "--mem",
            &options.get_memory_limit_kb().to_string()
        ]);

        if options.time_limit_s.is_some() {
            command.args(vec![
                "--time",
                &options.get_time_limit_s().to_string()
            ]);
        }

        if options.wall_time_limit_s.is_some() {
            command.args(vec![
                "--wall-time",
                &options.get_wall_time_limit_s().to_string()
            ]);
        }

        if options.extra_time_s.is_some() {
            command.args(vec![
                "--extra-time",
                &options.get_extra_time_s().to_string()
            ]);
        }

        match options.max_process {
            MaxProcessCount::Fixed(1) => {},
            MaxProcessCount::Fixed(x) => {
                command.arg(&format!("--processes={}", x));
            },
            MaxProcessCount::Unlimited => {
                command.arg("--processes");
            }
        }

        command.arg("--run");
        command.arg("--");
        command.arg(&actual_path(&program));
        command.args(args);

        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::from(stderr_file.get_file_write()));

        debug!("Launching command {:?} in sandbox {}", command, self.box_id);

        let child = command.spawn().unwrap();

        RunningJob::new(child, stderr_file, metafile_file, None)
    }

    pub fn box_dir(&self) -> &str {
        &self.box_path
    }
}

impl Drop for IsolateSandbox {
    fn drop(&mut self) {
        let fut = self.cleanup();
        async_std::task::block_on(fut);
    }
}