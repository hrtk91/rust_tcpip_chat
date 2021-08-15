mod rudecha;
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::Sender;
use std::collections::VecDeque;

enum Category {
    Send,
    Recv,
}

struct Message {
    category: Category,
    content: String, 
}

fn rewrite(queue: &VecDeque<Message>) {
    for thread_msg in queue.iter() {
        match thread_msg.category {
            Category::Send => {
                println!("me: {}", thread_msg.content);
            },
            Category::Recv => {
                println!("partner: {}", thread_msg.content);
            }
        };
    }
}

fn update_console(queue: &mut VecDeque<Message>, msg: Message) {
    queue.push_back(msg);

    if queue.len() > 10 {
        queue.pop_front();
    }
                    
    println!("{}", termion::clear::All);
    rewrite(&queue);
}

fn read_connection_info() -> (String, String) {
    println!("host:");
    let mut input = String::new();
    let host = match std::io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().to_string(),
        Err(e) => panic!("input failed: {:?}", e),
    };

    println!("port:");
    let mut input = String::new();
    let port = match std::io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().to_string(),
        Err(e) => panic!("input failed: {:?}", e),
    };

    (host, port)
}

fn read_user_input_loop(tx: Sender<String>) -> JoinHandle<()> {
    thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut buf = String::new();
            let user_input = match stdin.read_line(&mut buf) {
                Ok(_) => buf.trim(),
                Err(_) => continue,
            };
    
            tx.send(user_input.to_string()).unwrap();
    
            if let ":q" = user_input {
                return
            }
        }
    })
}

fn main_loop(mut rudecha: rudecha::RuDeCha) {
    let mut queue: VecDeque<Message> = VecDeque::new();
    println!("{}", termion::clear::All);
    let (tx, rx) = std::sync::mpsc::channel();
    let input_handle = read_user_input_loop(tx);

    rudecha.start().unwrap();

    loop {
        if let Ok(user_input) = rx.try_recv() {
            if user_input == ":q" {
                rudecha.shutdown().unwrap();
                input_handle.join().unwrap();
                return
            } else {
                rudecha.send(user_input.to_string());
                update_console(&mut queue, Message {
                    category: Category::Send,
                    content: user_input.to_string(),
                });
            }
        }

        let msg = match rudecha.recv() {
            Ok(msg) => msg,
            Err(e) if e == rudecha::RecvErr::Terminate => {
                println!("connection closed");
                println!("program shutdown");
                return
            },
            Err(_) /* if e == rudecha::RecvErr::Empty */ => continue,
        };

        update_console(&mut queue, Message {
            category: Category::Recv,
            content: msg,
        });
    }
}

fn main() {
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

    let rudecha: rudecha::RuDeCha = match input.as_str() {
        "connect" => {
            let (host, port) = read_connection_info();
            rudecha::RuDeCha::connect(&host, &port).unwrap()
        },
        "accept" => {
            let (host, port) = read_connection_info();
            match rudecha::RuDeCha::listen(&host, &port) {
                Ok(rudecha) => rudecha,
                Err(e) => panic!("{:?}", e),
            }
        },
        _ => return println!("input error"),
    };

    main_loop(rudecha);
}
