use std::thread::JoinHandle;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::io::{ Write, Read };

pub const HOST: &str = "[::]";
pub const PORT: u32 = 8080;
pub const TERMINATE_CODE: &str = "system:terminate";

#[derive(Debug)]
pub struct ThreadMessage {
    pub category: Category,
    pub content: String,
}

#[derive(PartialOrd, PartialEq, Debug)]
pub enum Category {
    Send,
    Receive,
    Terminate,
}

pub fn listen(host: String, port: u32) -> Result<(TcpStream, SocketAddr), String> {
    let listener = TcpListener::bind(format!("{}:{}", host, port));
    let tcp_listener = match listener {
        Ok(listener) => listener,
        Err(e) => return Err(format!("fault create listener: {:?}", e)),
    };

    match tcp_listener.accept() {
        Ok( (stream, addr) ) => Ok( (stream, addr) ),
        Err(e) => Err(format!("fault listener accept start: {:?}", e)),
    }
}

pub fn connect(dest: String, port: String) -> Result<TcpStream, String> {
    match TcpStream::connect(format!("{}:{}", dest, port)) {
        Ok(tcpstream) => Ok(tcpstream),
        Err(e) => Err(format!("failed connection: {:?}", e)),
    }
}

pub fn start(stream: TcpStream) -> (std::sync::mpsc::Receiver<ThreadMessage>, JoinHandle<()>, JoinHandle<()>) {
    let (tx, rx): (mpsc::Sender<ThreadMessage>, mpsc::Receiver<ThreadMessage>) = mpsc::channel();
    let clone_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => panic!("failed to clone stream: {:?}", e),
    };
    let receive_thread = receive_main(clone_stream, tx.clone());

    let clone_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => panic!("failed to clone stream: {:?}", e),
    };
    let send_thread = send_main(clone_stream, tx.clone());

    (rx, receive_thread, send_thread)
}

fn send_main(mut stream: TcpStream, tx: mpsc::Sender<ThreadMessage>) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let mut input: String = String::new();
            let size = std::io::stdin().read_line(&mut input);
            if let Err(e) = size {
                println!("send_main::read error: {:?}", e);
                return
            }
    
            let input = input.replace("\n", "").replace("\r", "");
            if input == TERMINATE_CODE {
                stream.shutdown(std::net::Shutdown::Both).unwrap();
                let thread_message = ThreadMessage {
                    category: Category::Terminate,
                    content: String::from(""),
                };
                if let Err(e) = tx.send(thread_message) {
                    panic!("fault send thread message: {:?}", e);
                }
                return
            }

            let bytes = input.as_bytes();
            let write_result = stream.write(bytes);
            if let Err(e) = write_result {
                println!("send_main::send error: {:?}", e);
            }
    
            // ここで送信内容をメインスレッドに送る
            let thread_msg = ThreadMessage {
                category: Category::Send,
                content: input
            };
            if let Err(e) = tx.send(thread_msg) {
                panic!("send_main::failed to send thread message: {:?}", e);
            }
        }
    })
}

fn receive_main(mut stream: TcpStream, tx: mpsc::Sender<ThreadMessage>) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let mut buf = [0u8; 1024];
            let size =  stream.read(&mut buf);
            if let Err(e) = size {
                println!("fault stream read: {:?}", e);
                return
            }
            
            let size = size.unwrap();
    
            if size == 0 {
                let thread_message = ThreadMessage {
                    category: Category::Terminate,
                    content: String::from(""),
                };
                if let Err(e) = tx.send(thread_message) {
                    panic!("fault send thread message: {:?}", e);
                }
                return
            }
    
            let res = String::from_utf8(buf[..size].to_vec());
            if let Err(e) = res {
                println!("fault convert response: {:?}", e);
                return
            }

            // ここでメインスレッドに受信内容送る
            let thread_msg = ThreadMessage {
                category: Category::Receive,
                content: res.unwrap(),
            };
            if let Err(e) = tx.send(thread_msg) {
                panic!("receive_main::failed to send thread message: {:?}", e);
            };
        }
    })
}
