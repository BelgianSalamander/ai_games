use async_std::io::{Error, Write, Read};
use std::future::Future;
use std::io::ErrorKind;
use std::path::Path;
use std::process::ExitStatus;
use std::{fmt::format, pin::Pin};
use std::fs::File;
use async_std::process::{Child, Command, Output, Stdio, ChildStdout, ChildStdin};
use std::sync::{Arc, Mutex};
use log::{debug, info, trace, warn};
use crate::util::temp_file::{SharedTempFile, TempFile};

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

#[derive(Copy, Clone)]
pub struct LaunchOptions {
    pub memory_limit_kb: Option<u32>,
    pub time_limit_s: Option<f32>,
    pub wall_time_limit_s: Option<f32>,
    pub extra_time_s: Option<f32>,
}

impl LaunchOptions {
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
}

pub struct LaunchOptionsBuilder {
    memory_limit_kb: Option<u32>,
    time_limit_s: Option<f32>,
    wall_time_limit_s: Option<f32>,
    extra_time_s: Option<f32>,
}

impl LaunchOptionsBuilder {
    pub fn new() -> LaunchOptionsBuilder {
        LaunchOptionsBuilder {
            memory_limit_kb: None,
            time_limit_s: None,
            wall_time_limit_s: None,
            extra_time_s: None,
        }
    }

    pub fn memory_limit_kb(mut self, memory_limit_kb: u32) -> LaunchOptionsBuilder {
        self.memory_limit_kb = Some(memory_limit_kb);
        self
    }

    pub fn time_limit_s(mut self, time_limit_s: f32) -> LaunchOptionsBuilder {
        self.time_limit_s = Some(time_limit_s);
        self
    }

    pub fn wall_time_limit_s(mut self, wall_time_limit_s: f32) -> LaunchOptionsBuilder {
        self.wall_time_limit_s = Some(wall_time_limit_s);
        self
    }

    pub fn extra_time_s(mut self, extra_time_s: f32) -> LaunchOptionsBuilder {
        self.extra_time_s = Some(extra_time_s);
        self
    }

    pub fn build(self) -> LaunchOptions {
        LaunchOptions {
            memory_limit_kb: self.memory_limit_kb,
            time_limit_s: self.time_limit_s,
            wall_time_limit_s: self.wall_time_limit_s,
            extra_time_s: self.extra_time_s,
        }
    }
}

pub struct LaunchInfo {
    pub child: Child,
    pub metafile_path: String,
}

pub struct RunningJob {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<ChildStdout>>,

    metafile: TempFile,
    stderr: TempFile,

    killed: bool,
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
                println!("Read {} bytes", bytes);
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
    pub fn new(mut child: Child, mut stderr: TempFile, metafile: TempFile) -> Self {
        let stdin = Arc::new(Mutex::new(child.stdin.take().unwrap()));
        let stdout = Arc::new(Mutex::new(child.stdout.take().unwrap()));

        //stderr.freeze();

        RunningJob {
            child,
            stdin,
            stdout,
            metafile,
            stderr,

            killed: false,
        }
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

    pub async fn wait(&mut self) -> Result<(), Error> {
        match self.child.status().await {
            Ok(x) => {
                if !x.success() {
                    println!("Child exited with status {}", x);
                }

                Ok(())
            },
            Err(e) => Err(e)
        }
    }

    pub async fn kill(&mut self) -> Result<(), Error> {
        if self.killed {
            return Ok(());
        }

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

        Ok(())
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
        let mut command = Command::new("isolate");

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
        let mut command = Command::new("isolate");

        command.arg("--cleanup");
        command.arg("--box-id");
        command.arg(self.box_id.to_string());

        panic_on_fail(&mut command).await;
    }

    pub fn launch(
        &self,
        program: String, args: Vec<String>, 
        mapped_dirs: Vec<(String, String)>,
        env_vars: Vec<(String, String)>,
        stdin_file: Option<String>, options: &LaunchOptions
    ) -> RunningJob {
        trace!("Launching command {} in sandbox {}", program, self.box_id);
        let mut command = Command::new("isolate");

        let metafile_file = TempFile::with_extra(".meta");
        let stderr_file = TempFile::with_extra(".stderr");
        

        command.args(vec![
            "--box-id",
            &self.box_id.to_string(),
            format!("--meta={}", actual_path(&metafile_file.path)).as_str(),
        ]);

        for (host_dir, box_dir) in mapped_dirs {
            trace!("Binding directory {} to {}", &actual_path(&host_dir), &box_dir);
            command.args(vec![
                "--dir",
                format!("{}={}", box_dir, &actual_path(&host_dir)).as_str()
            ]);
        }

        for (key, value) in env_vars {
            trace!("Setting environment variable {} to {}", key, value);
            command.args(vec![
                "--env",
                format!("{}={}", key, value).as_str()
            ]);
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

        command.arg("--run");
        command.arg(program);
        command.args(args);

        if let Some(stdin_file) = stdin_file {
            command.stdin(Stdio::from(File::open(&stdin_file).unwrap()));
        } else {
            command.stdin(Stdio::piped());
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::from(stderr_file.get_file_write()));

        debug!("Launching command {:?} in sandbox {}", command, self.box_id);

        let child = command.spawn().unwrap();

        RunningJob::new(child, stderr_file, metafile_file)
    }
}

impl Drop for IsolateSandbox {
    fn drop(&mut self) {
        let fut = self.cleanup();
        async_std::task::block_on(fut);
    }
}