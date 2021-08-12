use std::net::{ TcpListener, TcpStream };
use std::io::{ Read, Write };
use std::thread;
use std::thread::{ JoinHandle };
// use std::env;
use std::sync::mpsc;

const HOST: &str = "[::]";
const PORT: &str = "8080";

#[derive(Debug)]
struct ThreadMessage {
    category: Category,
    content: String,
}

#[derive(PartialOrd, PartialEq, Debug)]
enum Category {
    Send,
    Receive,
    Terminate,
}

const TERMINATE_CODE: &str = "system:terminate";

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

fn rewrite(queue: &std::collections::VecDeque<ThreadMessage>) {
    for thread_msg in queue.iter() {
        match thread_msg.category {
            Category::Send => {
                println!("me: {}", thread_msg.content);
            },
            Category::Receive => {
                println!("partner: {}", thread_msg.content);
            },
            _ => { }
        };
    }
}

fn main_loop(rx: mpsc::Receiver<ThreadMessage>) {
    use std::collections::VecDeque;
    let mut queue: VecDeque<ThreadMessage> = std::collections::VecDeque::new();
    println!("{}", termion::clear::All);
    loop {
        let thread_msg = match rx.recv() {
            Ok(msg) => match msg.category {
                Category::Terminate => {
                    println!("connection terminated");
                    println!("program shutdown");
                    return
                },
                _ => msg,
            },
            Err(e) => panic!("cannot receive thread message: {:?}", e),
        };

        queue.push_back(thread_msg);
        if queue.len() > 10 {
            queue.pop_front();
        };
        println!("{}", termion::clear::All);
        rewrite(&queue);
    }
}

fn main() {
    // let args: Vec<String> = env::args().collect();

    println!("program start");

    let input = loop {
        println!("connect or accept:");
        let mut input = String::new();
        let result = std::io::stdin().read_line(&mut input);
        if let Ok(_) = result {
            break input.trim().to_string();
        } else {
            continue;
        }
    };

    let stream: TcpStream = if input == "accept" {
        let listener = TcpListener::bind(format!("{}:{}", HOST, PORT));
        if let Err(e) = listener {
            println!("fault create listener: {:?}", e);
            return
        }
        let tcp_listener = listener.unwrap();

        println!("connection waiting...");
        let accept_result = tcp_listener.accept();
        if let Err(e) = accept_result {
            println!("fault listener accept start: {:?}", e);
            return
        }
        let (tcpstream, addr) = accept_result.unwrap();
        println!("connect to {}", addr);
        tcpstream
    } else {
        println!("connect to:");
        let mut input = String::new();
        let destination = match std::io::stdin().read_line(&mut input) {
            Ok(_) => input.trim(),
            Err(e) => {
                println!("cannot connection: {:?}", e);
                return
            },
        };

        match TcpStream::connect(destination) {
            Ok(tcpstream) => tcpstream,
            Err(e) => {
                println!("failed connection: {:?}", e);
                return
            },
        }
    };

    let (tx, rx): (mpsc::Sender<ThreadMessage>, mpsc::Receiver<ThreadMessage>) = mpsc::channel();
    let clone_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => panic!("failed to clone stream: {:?}", e),
    };
    receive_main(clone_stream, tx.clone());

    let clone_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => panic!("failed to clone stream: {:?}", e),
    };
    send_main(clone_stream, tx.clone());

    main_loop(rx);
}
