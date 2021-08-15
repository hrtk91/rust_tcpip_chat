use std::thread::JoinHandle;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::io::{ Write, Read };
use std::sync::mpsc::{ Receiver, Sender, TryRecvError };

pub const HOST: &str = "[::]";
pub const PORT: &str = "8080";

#[derive(Debug)]
struct ThreadMessage {
    pub category: Category,
    pub content: String,
}

#[derive(PartialOrd, PartialEq, Debug)]
enum Category {
    Receive,
    Terminate,
}

#[derive(Debug)]
struct Messenger<T> {
    pub tx: Sender<T>,
    pub rx: Receiver<T>,
}

#[derive(PartialOrd, PartialEq, Debug)]
pub enum RecvErr {
    Empty,
    Terminate,
}

#[derive(Debug)]
pub struct RuDeCha {
    stream: TcpStream,
    thread_handle: Option<JoinHandle<()>>,
    messenger: Option<Messenger<ThreadMessage>>,
}

impl RuDeCha {
    pub fn listen(host: &str, port: &str) -> Result<RuDeCha, String> {
        let host = match host {
            "" => HOST,
            _ => host,
        };
        let port = match port {
            "" => PORT,
            _ => port,
        };
        let listener = match TcpListener::bind(format!("{}:{}", host, port)) {
            Ok(listener) => listener,
            Err(e) => return Err(format!("fault create listener: {:?}", e)),
        };
    
        match listener.accept() {
            Ok( (stream, _peer) ) => {
                Ok(RuDeCha {
                    stream: stream,
                    thread_handle: None,
                    messenger: None,
                })
            },
            Err(e) => Err(format!("fault listener accept start: {:?}", e)),
        }
    }

    pub fn connect(dest: &str, port:&str) -> Result<RuDeCha, String> {
        match TcpStream::connect(format!("{}:{}", dest, port)) {
            Ok(tcpstream) => {
                Ok(RuDeCha {
                    stream: tcpstream,
                    thread_handle: None,
                    messenger: None,
                })
            },
            Err(e) => Err(format!("failed connection: {:?}", e)),
        }
    }

    pub fn send(&mut self, message: String) -> usize {
        self.stream.write(message.as_bytes()).unwrap()
    }
    
    pub fn recv(&self) -> Result<String, RecvErr> {
        if let None = self.messenger {
            panic!("messenger not found")
        }
        let messenger = self.messenger.as_ref().unwrap();

        let msg = match messenger.rx.try_recv() {
            Ok(msg) if msg.category == Category::Terminate => return Err(RecvErr::Terminate),
            Ok(msg) => msg.content,
            Err(_) => return Err(RecvErr::Empty),
        };

        Ok(msg)
    }

    pub fn start(&mut self) -> Result<(), String> {
        let (for_receive_thread, for_self) = RuDeCha::create_messenger();
        self.thread_handle = match RuDeCha::receive_main(self.stream.try_clone().unwrap(), for_receive_thread) {
            Ok(thread_handle) => Some(thread_handle),
            Err(e) => return Err(format!("RuDeCha::start() : {:?}", e)),
        };
        self.messenger = Some(for_self);
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        if let None = self.messenger {
            return Err("not started".to_string())
        }
        if let None = self.thread_handle {
            return Err("not started".to_string());
        }
        let messenger = self.messenger.take().unwrap();
        let handle = self.thread_handle.take().unwrap();

        match messenger.tx.send(ThreadMessage {
            category: Category::Terminate,
            content: String::new(),
        }) {
            Ok(_) => {
                self.stream.shutdown(std::net::Shutdown::Both).unwrap();
                handle.join().unwrap();
                Ok(())
            },
            Err(e) => return Err(format!("{:?}", e)),
        }
    }

    fn receive_main(stream: TcpStream, messenger: Messenger<ThreadMessage>) -> Result<JoinHandle<()>, String> {
        let mut stream = stream.try_clone().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_millis(32))).unwrap();

        Ok(thread::spawn(move || {
            let mut buff = [0u8; 1024];
            loop {
                // メインスレッドから停止命令きたら停止
                match messenger.rx.try_recv() {
                    Ok(thread_msg) if thread_msg.category == Category::Terminate => {
                        return
                    },
                    Ok(_) => { /* なにもしない */},
                    Err(e) if e == TryRecvError::Disconnected =>
                        println!("RuDeCha::receive_main() : messenger disconnected. shutdown program"),
                    Err(_) /* if e == TryRecvError::Empty */ => { /* なにもしない */ },
                };

                // 受信したらメインスレッドに送信
                let size = match stream.read(&mut buff) {
                    Ok(size) => size,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => panic!("RuDeCha::receive_main() : stream read error"),
                };

                if size == 0 {
                    // 切断の合図なので終了を送信
                    messenger.tx.send(ThreadMessage { category: Category::Terminate, content: "".to_string() }).unwrap();
                    return
                } else {
                    messenger.tx.send(ThreadMessage {
                        category: Category::Receive,
                        content: String::from_utf8(buff.to_vec()).unwrap(),
                    }).unwrap();
                }
            }
        }))
    }

    fn create_messenger() -> (Messenger<ThreadMessage>, Messenger<ThreadMessage>) {
        let (tx1, rx1): (Sender<ThreadMessage>, Receiver<ThreadMessage>) = mpsc::channel();
        let (tx2, rx2): (Sender<ThreadMessage>, Receiver<ThreadMessage>) = mpsc::channel();
        let messenger1 = Messenger {
            tx: tx1,
            rx: rx2,
        };
        let messenger2 = Messenger {
            tx: tx2,
            rx: rx1,
        };
        (messenger1, messenger2)
    }
}
