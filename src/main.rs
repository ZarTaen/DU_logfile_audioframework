use std::path::PathBuf;
use std::ffi::OsString;
use std::time::{Duration, UNIX_EPOCH};
use std::io::{BufReader, SeekFrom, Seek, BufRead, Write, stdin, stdout};
use std::fs::File;
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::sleep;
use app_dirs::{get_app_dir, AppDataType, AppInfo};
use notify::{RawEvent, Op, raw_watcher, RecursiveMode, Watcher};
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;
use config::Config;
use std::sync::Arc;
use std::sync::atomic::Ordering::{Acquire, Release};
use rodio::{OutputStream, Sink, Decoder};

///Input Handling for shutting down the program.
fn end_this_world(electric_atomic_seppuku:Arc<AtomicBool>){
    loop {
        let mut s= String::new();
        print!("To end the program, press y and confirm with enter: \n");
        let _=stdout().flush();
        match stdin().read_line(&mut s) {
            Ok(_) => {
                match s.as_bytes()[0] {
                    121 => {
                        break;
                    }
                    89 => {
                        break;
                    }
                    _ => {}
                }
            }
            Err(_) => {}
        };
    }
    electric_atomic_seppuku.store(true, Release);
}

///Gets the most recent Logfile
fn most_recent_file(path:PathBuf) -> (OsString, Duration){
    let mut old_time = (OsString::new(), Duration::from_micros(0));
    let entries = path.read_dir().expect("");
    for entry in entries {
        match entry{
            Ok(t) => {
                let metadata = match t.metadata(){
                    Ok(j) => {
                        match j.modified() {
                            Ok(k) => { k }
                            Err(_) => {continue}
                        }
                    }
                    Err(_) => {continue}
                };
                let diff = metadata.duration_since(UNIX_EPOCH).unwrap();
                if old_time.1<diff {
                    old_time.0 = t.file_name();
                    old_time.1 = diff;
                }
            }
            Err(_) => {}
        }
    }
    old_time
}

///Sender is for target Thread that works the String, file is the logfile, watcher is the watcher.
///This function looks for the Log Message ID that tells whether it was manually written.
fn log_reader(to_thread:Sender<String>, mut file:BufReader<File>, watcher_receiver:Receiver<RawEvent>, mut path:PathBuf, electric_atomic_seppuku:Arc<AtomicBool>) -> bool {
    let mut last_position = 0;
    let mut line_buffer = String::new();
    let mut return_value = false;
    loop {
        if electric_atomic_seppuku.load(Acquire){
            return_value = true;
            break;
        }
        match file.read_line( &mut line_buffer){
            Ok(t) => {
                if t == 0 {
                    sleep(Duration::from_millis(40));
                    match watcher_receiver.try_recv() {
                        Ok(t) => {
                            match check_rawevent(t){
                                None => {continue}
                                Some(t) => {
                                    if t!= path {
                                        path = t;
                                        file = BufReader::new(File::open(path.clone()).expect("Cannot open file"));
                                        file.seek(SeekFrom::End(0));
                                    }
                                }
                            };
                        }
                        Err(_) => {}
                    }
                }
                if t>31 {
                    if &line_buffer[0..20] == "<message>4176790050|" {
                        if &line_buffer[(line_buffer.len()-13)..line_buffer.len()-2] != r#"|</message>"# {
                            file.read_line(&mut line_buffer);
                            while &line_buffer[(line_buffer.len()-13)..line_buffer.len()-2] != r#"|</message>"# {
                                file.seek(SeekFrom::Start(last_position));
                                line_buffer = String::new();
                                file.read_line(&mut line_buffer);
                                if &line_buffer[(line_buffer.len()-13)..line_buffer.len()-2] != r#"|</message>"# {
                                    file.read_line(&mut line_buffer);
                                }
                            }
                            //println!("\n Linebuffer:{}\n\n",line_buffer);
                        }
                        to_thread.send(line_buffer);
                    }
                }
            }
            Err(_) => {}
        };
        last_position = file.seek(SeekFrom::Current(0)).unwrap();
        line_buffer = "".to_string();
    }
    return_value
}

///Uh..This was related to the watcher, I think it replies the path whenever a file is written to or created..
fn check_rawevent(t:RawEvent) -> Option<PathBuf> {
    match t.op{
        Ok(j)=> {
            match j {
                Op::WRITE => {
                    t.path
                },
                Op::CREATE => {
                    t.path
                },
                _ => None
            }
        }
        _ => None
    }
}

fn main() {
    //Getting the settings.
    let mut config = Config::default();
    config.merge(config::File::with_name("conf/conf.toml")).unwrap();
    let mut settings_names = vec![];
    settings_names.push("prio_count");
    settings_names.push("channel_count");
    settings_names.push("priority_volume");
    settings_names.push("channel_volume");

    let settings = match config.try_into::<HashMap<String,u8>>(){
        Ok(t) => {
            for name in settings_names{
                match t.contains_key(name){
                    true => {
                        continue
                    }
                    false => {
                        panic!("There is no {} in conf.toml.",name);
                    }
                }
            }
            t
        }
        Err(e) => {
            panic!("Something went wrong with the conf.toml: {}",e);
        }
    };
    //The path is found via AppInfo, a crate that allows searching for a folder inside the AppData, because it is a different path
    //for each user.
    let game_string = "DualUniverse";
    let author_string = "NQ";
    let program_info = AppInfo{name: game_string, author: author_string};
    let mut path = get_app_dir(AppDataType::UserCache, &program_info, r#"log"#).expect("Directory does not exist.");
    //Here we found the directory.

    //Here we create channels so the watcher can do his magic.
    let (tx, rx) = channel();
    let mut watcher = raw_watcher(tx).unwrap();
    watcher.watch(path.clone(), RecursiveMode::NonRecursive).unwrap();
    //Watcher is watching and sending what he finds.

    let (to_thread, thread_recv):(Sender<String>,Receiver<String>) = channel();
    let electric_atomic_seppuku  = Arc::new(AtomicBool::new(false));
    let (new_audio_file_send,new_audio_file_receive) = channel();

    //finds the most recent file and assigns the Path accordingly.
    let most_recent_file = most_recent_file(path.clone()).0;
    path.push(PathBuf::from(&most_recent_file));

    //Opens file and initialises to EOF, in order to avoid old entries.
    let mut file = BufReader::new(File::open(path.clone()).expect("Cannot open file./No file available."));
    file.seek(SeekFrom::End(0));

    //generating some more atomicbool references, one for each thread.
    let electric_atomic_seppuku2 = electric_atomic_seppuku.clone();
    let electric_atomic_seppuku3 = electric_atomic_seppuku.clone();
    let electric_atomic_seppuku4 = electric_atomic_seppuku.clone();

    //Initialises the thread that receives the log entries.
    let johnny = thread::spawn(move || {
        worker(thread_recv, new_audio_file_send, electric_atomic_seppuku4)
    });

    //Initialises the thread that reads the log.
    let log_read_thread = thread::spawn(move || {
        log_reader(to_thread, file, rx, path.clone(), electric_atomic_seppuku2)
    });

    let audio_thread = thread::spawn(move || {
        audio_handling(new_audio_file_receive, *settings.get("prio_count").unwrap(), *settings.get("channel_count").unwrap(),
                       *settings.get("priority_volume").unwrap() as f32 /100.0, *settings.get("channel_volume").unwrap() as f32 /100.0, electric_atomic_seppuku3)
    });

    end_this_world(electric_atomic_seppuku);


    //This is done so we know whether a thread got killed by Error and never really recovered from his alcoholism.
    let mut thread_happiness = vec![];
    thread_happiness.push(johnny.join());
    thread_happiness.push(log_read_thread.join());
    thread_happiness.push(audio_thread.join());

    let mut overall_happiness = true;
    for i in thread_happiness{
        match i {
            Ok(t) => {
                if !t {
                    overall_happiness = false;
                    break;
                }
            }
            Err(_) => {
                overall_happiness = false;
                break;
            }
        }
    }

    if overall_happiness {
        println!("Shutdown was graceful.");
    }else {
        println!("One of the threads was not happy on shutdown.");
    }
}

///The worker does a lot of things, but mostly decides what happens with the data that is received from the logfile.
fn worker(thread_recv:Receiver<String>, audio_path_send:Sender<(String,u8)>, electric_atomic_seppuku:Arc<AtomicBool>) -> bool{
    let mut config = Config::default();
    config.merge(config::File::with_name("conf/audiopacks.toml")).unwrap();
    let mut return_value = false;

    let audiopacks = config.try_into::<HashMap<String, String>>().unwrap();

    loop {
        if electric_atomic_seppuku.load(Acquire){
            return_value = true;
            break;
        }
        let original_string = match thread_recv.recv() {
            Ok(t) => { t }
            Err(_) => {
                if electric_atomic_seppuku.load(Acquire){
                    return_value = true;
                }
                break;
            }
        };
        let mut cleaned_string = original_string[20..(original_string.len() - 13)].replace(r#"&quot;"#, r#"""#);
        cleaned_string = cleaned_string[1..cleaned_string.len() - 1].to_string();
        let mut strings = vec![];
        for str in cleaned_string.split('|'){
            strings.push(str.to_string());
        };
        let var_amount = strings.len();
        let modus = strings[0].to_string();

        //handled like this, in order to allow for more modes later and making it extensible in some way,
        //albeit not without a lot of more work
        match modus.as_str() {
            "audioplayback" => {
                if var_amount != 4{
                    println!("Not the right amount of arguments for an Audiopack Call. {:#?}", strings);
                    continue
                }
                //println!("Audioplayback entered.");
                if audiopacks.contains_key(&strings[1]){
                    let mut audiopack_path= audiopacks.get(&strings[1]).unwrap().clone();
                    let audio_file = strings[2].to_string();
                    let audio_prio:u8 = match strings[3].to_string().parse(){
                        Ok(t) => {t},
                        Err(_) => {
                            continue
                        },
                    };
                    audiopack_path.push('/');
                    audiopack_path.push_str(audio_file.as_str());
                    //println!("Audiopack_path: {}",audiopack_path);
                    audio_path_send.send((audiopack_path, audio_prio));
                }else {
                    println!("Not a valid Audiopack. {:#?}", strings);
                }
            },
            _ => {}
        }
    }
    return_value
}

///prio_count can be any number. It should be a low number.
///The idea is that the function determining the audiofile to play, then gives this thread the path.
///A Prio_count of 2 means 2 sinks, with valid inputs between of 1,2.
fn audio_handling(new_audio_file_recv:Receiver<(String, u8)>, prio_count: u8, channel_count:u8, priority_volume: f32, channel_volume: f32, electric_atomic_seppuku:Arc<AtomicBool>) -> bool{
    let timeout_duration = Duration::from_millis(50);
    let mut return_value = false;
    //Initialising Queues.
    let mut queue_list = vec![];
    let mut channel_list = vec![];
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    for _ in 0..prio_count{
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(priority_volume);
        queue_list.push(sink);
    }

    for _ in 0..channel_count{
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(channel_volume);
        channel_list.push(sink);
    }

    loop{
        let mut queue_bool = true;
        //This checks for the end of the program via command.
        if electric_atomic_seppuku.load(Acquire){
            for i in queue_list{
                i.stop();
                i.detach();
            }
            return_value = true;
            println!("Audio has been stopped.");
            break;
        }

        //this receives the audio via timeout, in order to allow queue repriorisation, to avoid a deadlock.
        let current_new = match new_audio_file_recv.recv_timeout(timeout_duration){
            Ok(t) => {
                match t.1{
                    0 => {
                        for i in 0..queue_list.len(){
                            let sink = Sink::try_new(&stream_handle).unwrap();
                            sink.set_volume(priority_volume);
                            queue_list[i] = sink;
                        }
                        for i in 0..channel_list.len(){
                            let sink = Sink::try_new(&stream_handle).unwrap();
                            sink.set_volume(channel_volume);
                            channel_list[i] = sink;
                        }
                        continue
                    },
                    1 => {
                        queue_bool = false;
                        t
                    }
                    x if x>(prio_count+1) => {
                        println!("Invalid Prio {} over Prio-range {}.",x, prio_count);
                        continue
                    },
                    _ =>{
                        //the lowest priority queue is at 2, but the queue_list starts at 0, thats why the correction.
                        (t.0, t.1-2)
                    }
                }
            }
            Err(_) => {
                reorder(&queue_list, prio_count.clone());
                continue
            }
        };

        let audio_file = BufReader::new(match File::open(current_new.0){
            Ok(t) => {t}
            Err(e) => {
                println!("{}",e);
                continue
            }
        });
        let source = match Decoder::new(audio_file){
            Ok(t) => { t }
            Err(_) => {
                println!("Decoder died, is this an audiofile?");
                continue
            }
        };

        if queue_bool {
            //If queue, then queue. Queue will be reordered at the end of the loop.
            queue_list[current_new.1 as usize].append(source);
            queue_list[current_new.1 as usize].play();
            //println!("Added to Priority: {}", current_new.1+1);

        }else{
            //The first is the channel number, the second is the channel len. Default is channel 0 with channel0 len.
            //In an ideal world, at most one sound is queued, if a long sound is being played. As long as channels are free, this should
            //result in no queued sounds. Once all channels are full, the first channel will be used, as it has a high likelihood of being finished first
            //for a similar length sound. I do not recommend queueing music tracks here, unless there are enough channels, and even then its iffy.
            let mut check_tuple:(usize, usize) = (0,channel_list[0].len());
            for i  in 1..channel_count as usize{
                let chan_len = channel_list[i].len();
                //should the current channel have a smaller chan_len, it will be saved into the tuple.
                if check_tuple.1 > chan_len{
                    check_tuple = (i, chan_len);
                }
            }
            //println!("Added to Channel: {}", check_tuple.0 +1);
            channel_list[check_tuple.0].append(source);
            channel_list[check_tuple.0].play();
        }
        reorder(&queue_list, prio_count.clone())
    }
    return_value
}

///Reorders the Priority Queues, so higher priority is actually higher priority.
fn reorder(queue_list:&Vec<Sink>, prio_count:u8){
    //goes through the sinks and the early sinks are prioritised in playback.
    let mut toggle = false;
    for i in 0..prio_count{
        if toggle{
            queue_list[i as usize].pause();
        }else{
            if !queue_list[i as usize].empty(){
                queue_list[i as usize].play();
                toggle = true;
            }
        }
    }
}