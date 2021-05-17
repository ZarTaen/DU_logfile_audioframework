use std::path::PathBuf;
use std::ffi::OsString;
use std::time::{Duration, UNIX_EPOCH, Instant};
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
use rodio::{OutputStream, Sink, Decoder, OutputStreamHandle};

#[derive(PartialEq, Eq)]
enum SoundCommand{
    Play,
    Notification,
    Queue,
    Volume,
    Pause,
    Stop,
    Resume
}

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

///Gets the most recent Logfile initially
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
                    let result = file_watching(file, &watcher_receiver, path);
                    file = result.0;
                    path = result.1;
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
            Err(_) => {
                let result = file_watching(file, &watcher_receiver, path);
                file = result.0;
                path = result.1;
            }
        };
        last_position = file.seek(SeekFrom::Current(0)).unwrap();
        line_buffer = "".to_string();
    }
    return_value
}

///Either returns the file unchanged, or returns an updated file from an updated Path.
///Should the readline have failed, then the watcher would see when a new.
fn file_watching(mut file:BufReader<File>, watcher_receiver:&Receiver<RawEvent>, mut path:PathBuf) -> (BufReader<File>, PathBuf){
    //sleep(Duration::from_millis(10));
    sleep(Duration::from_millis(10));
    match watcher_receiver.try_recv() {
        Ok(t) => {
            match check_rawevent(t){
                None => {return (file, path)}
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
    (file, path)
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
    match config.merge(config::File::with_name("conf/conf.toml")){
        Ok(_) => {}
        Err(_) => {
            //this happens if a file doesnt exist -> so it has to be generated, right?
            //TODO: Generate the conf.toml with default values.

        }
    }
    let mut settings_names = vec![];
    settings_names.push("notification_volume");
    settings_names.push("concurrent_volume");
    settings_names.push("queue_volume");
    settings_names.push("notification_difference");

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

    let notification_volume = *settings.get("notification_volume").unwrap() as f32 /100.0;
    let concurrent_volume = *settings.get("concurrent_volume").unwrap() as f32 /100.0;
    let queue_volume = *settings.get("queue_volume").unwrap() as f32 /100.0;

    //Initialises the thread that receives the log entries.
    let johnny = thread::spawn(move || {
        worker(thread_recv, new_audio_file_send, electric_atomic_seppuku4, notification_volume.clone(),
               concurrent_volume.clone(), queue_volume.clone())
    });

    //Initialises the thread that reads the log.
    let log_read_thread = thread::spawn(move || {
        log_reader(to_thread, file, rx, path.clone(), electric_atomic_seppuku2)
    });

    let audio_thread = thread::spawn(move || {
        audio_handling(new_audio_file_receive, electric_atomic_seppuku3, *settings.get("notification_difference").unwrap() as f32 /100.0)
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
fn worker(thread_recv:Receiver<String>, audio_path_send:Sender<(SoundCommand, String, String, f32)>, electric_atomic_seppuku:Arc<AtomicBool>, concurrent_volume: f32, notification_volume: f32, queue_volume: f32 ) -> bool{
    let mut return_value = false;
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
            "sound_play" => {
                file_to_audio_handling(SoundCommand::Play, var_amount, strings, &audio_path_send, concurrent_volume.clone());
            },
            "sound_notification" => {
                file_to_audio_handling(SoundCommand::Notification, var_amount, strings, &audio_path_send, notification_volume.clone());
            },
            "sound_q" => {
                file_to_audio_handling(SoundCommand::Queue, var_amount, strings, &audio_path_send, queue_volume.clone());
            },
            "sound_volume" => {
                control_to_audio_handling(SoundCommand::Volume, var_amount, strings, &audio_path_send);
            },
            "sound_pause" => {
                control_to_audio_handling(SoundCommand::Pause, var_amount, strings, &audio_path_send);
            },
            "sound_stop" => {
                control_to_audio_handling(SoundCommand::Stop, var_amount, strings, &audio_path_send);
            },
            "sound_resume" => {
                control_to_audio_handling(SoundCommand::Resume, var_amount, strings, &audio_path_send);
            }
            _ => {}
        }
    }
    return_value
}

///Handles the volume parsing and therefore what volume is used, and then sends it to the audiohandler
fn file_to_audio_handling(command: SoundCommand, var_amount: usize, strings: Vec<String>, audio_path_send:&Sender<(SoundCommand, String, String, f32)>, default_volume:f32){
    let vol;
    match var_amount {
        3 => { //No Volume specified, default will be used.
            vol = default_volume;
        },
        4 => { //Volume specified.
            vol = match strings[3].to_string().parse::<u8>(){
                Ok(mut t) => {
                    if t >100{
                        t = 100;
                    }
                    (t as f32/100.0)*default_volume
                }
                Err(_) => {
                    println!("Wrong argument or invalid range for volume, default volume used.");
                    default_volume.clone()
                }
            };
        },
        _ => { //Invalid amount of arguments.
            println!("Not the right amount of arguments for an audiofile playback. {:#?}", strings);
            return
        }
    }
    audio_path_send.send((command, strings[1].to_string(), strings[2].to_string(), vol));
}

fn control_to_audio_handling(command: SoundCommand, var_amount: usize, strings: Vec<String>, audio_path_send:&Sender<(SoundCommand, String, String, f32)>){
    let mut vol = 0 as f32;
    let string1;
    match var_amount {
        x if x==1 ||x==2 => {
            if command == SoundCommand::Volume {
                println!("An argument is missing for volume.");
                return
            }
            if x == 2 {
                string1 = strings[1].to_string();
            }else {
                string1 = "".to_string();
            }
        },
        3 => {
            if command != SoundCommand::Volume {
                println!("There are too many arguments.");
                return
            }
            vol = match strings[2].to_string().parse::<u8>(){
                Ok(mut t) => {
                    if t >100{
                        t = 100;
                    }
                    t as f32/100.0
                }
                Err(_) => {
                    println!("Wrong argument or invalid range for volume, no changes.");
                    return
                }
            };
            string1 = strings[1].to_string();
        }
        _ => {return}
    }
    audio_path_send.send((command,  "".to_string(), string1, vol));
}

struct AudioEntry {
    sound_command: SoundCommand,
    volume:f32,
    path: String,
    sink: Sink,
    pause_state: bool,
}

///I dont feel like rewriting this, but here goes: This is the function to handle all the audio management needs, be it play, notification, queue, stop, resume, pause, etc.
fn audio_handling(new_audio_file_recv:Receiver<(SoundCommand, String, String, f32)>, electric_atomic_seppuku:Arc<AtomicBool>, notification_difference: f32) -> bool{
    let timeout_duration = Duration::from_millis(10);
    let mut return_value = false;
    //The modifier applied each update for non-notification sinks.
    let mut active_notification_modifier = 1.0 as f32;
    //The ID map for every soundfile.
    let mut sound_map: HashMap<String, AudioEntry> = HashMap::new();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    //The stacks keep IDs, nothing else.
    let mut queue_stack = vec![];
    let mut notification_stack = vec![];

    let mut notification_sink = Sink::try_new(&stream_handle).unwrap();
    let mut queue_sink = Sink::try_new(&stream_handle).unwrap();

    //The paused_vecs take tuples with the id at 0 and the paused sink at 1.
    let mut paused_notifications:Vec<(String, Sink)> = vec![];
    let mut paused_queue:Vec<(String, Sink)> = vec![];

    let mut active_notification= false;
    let mut active_queue = false;
    let mut discard_on_load_notification = false;
    let mut discard_on_load_queue = false;

    loop{

        //This checks for the end of the program via command.
        if electric_atomic_seppuku.load(Acquire){
            queue_sink.stop();
            queue_sink.detach();
            notification_sink.stop();
            notification_sink.detach();

            for i in &mut sound_map {
                i.1.sink.stop();
            }
            for i in paused_notifications {
                i.1.stop();
                i.1.detach();
            }
            for i in paused_queue {
                i.1.stop();
                i.1.detach();
            }

            return_value = true;
            println!("Audio has been stopped.");
            break;
        }

        //this receives the audio via timeout, in order to allow queue repriorisation, to avoid a deadlock.
        match new_audio_file_recv.recv_timeout(timeout_duration){
            Ok(t) => {
                match t.0{
                    SoundCommand::Play => {
                        let source = match open_audio_file(t.1.clone()){
                            Ok(t) => {
                                t
                            },
                            Err(e) => {
                                println!("{}",e);
                                continue
                            }
                        };
                        if sound_map.contains_key(t.2.as_str()){
                            let entry = sound_map.get_mut(t.2.as_str()).unwrap();
                            entry.volume = t.3;
                            entry.path = t.1;
                            entry.sound_command = t.0;
                            entry.sink.stop();
                            entry.sink = Sink::try_new(&stream_handle).unwrap();
                            entry.sink.set_volume(entry.volume*active_notification_modifier);
                            entry.sink.append(source);
                            entry.sink.play();
                        }else{
                            //creates a sink that plays the source and adds an entry with default volume for the entry, the path, and the type of playback it adheres to.
                            let sink = Sink::try_new(&stream_handle).unwrap();
                            let entry = AudioEntry {
                                sound_command: t.0,
                                volume: t.3,
                                path: t.1,
                                sink: sink,
                                pause_state:false
                            };
                            //queues itself to its own sink, lol
                            entry.sink.set_volume(entry.volume*active_notification_modifier);
                            entry.sink.append(source);
                            entry.sink.play();
                            sound_map.insert(t.2, entry);
                        };
                    }
                    SoundCommand::Notification => {
                        discard_on_load_notification = queue_decision(t, &mut sound_map, &mut notification_stack, &mut paused_notifications, &notification_sink, discard_on_load_notification, &stream_handle);
                    }
                    SoundCommand::Queue => {
                        discard_on_load_queue = queue_decision(t, &mut sound_map, &mut queue_stack, &mut paused_queue,&notification_sink , discard_on_load_queue, &stream_handle);
                    }
                    //The volume will be set again in the loop below anyway.
                    SoundCommand::Volume => {
                        match sound_map.get_mut(t.2.as_str()){
                            None => {
                                println!("The ID does not exist.");
                            }
                            Some(k) => {
                                match k.sound_command{
                                    SoundCommand::Play => {
                                        k.volume = t.3;
                                        k.sink.set_volume(k.volume*active_notification_modifier);
                                    }
                                    SoundCommand::Notification => {
                                        k.volume = t.3;
                                        if notification_stack[0].as_str() == t.2.as_str(){
                                            notification_sink.set_volume(k.volume);
                                        }
                                    }
                                    SoundCommand::Queue => {
                                        k.volume = t.3;
                                        if queue_stack[0].as_str() == t.2.as_str(){
                                            queue_sink.set_volume(t.3*active_notification_modifier);
                                        }
                                    }
                                    _ => {
                                        println!("bruh, you gotta explain this");
                                    }
                                }
                            }
                        };
                    }
                    SoundCommand::Pause => {
                        if t.2.as_str() == "" {
                            for i in &mut sound_map {
                                i.1.sink.pause();
                                i.1.pause_state = true;
                            }

                            if notification_stack.len()>0 {
                                //first thing, the currently active notification is pushed into the paused_notifications.oof.jpeg.gif.bat.tif
                                notification_sink.pause();
                                paused_notifications.push((notification_stack[0].clone(),notification_sink));
                                notification_stack.remove(0);
                                //it is then removed.
                                notification_sink = Sink::try_new(&stream_handle).unwrap();
                                //a new sink is created.

                                //and then, every single entry inside notification_stack is pushed into the paused_notifications, by order.
                                while notification_stack.len()>0{
                                    let entry = sound_map.get(notification_stack[0].as_str()).unwrap();
                                    //the entry is gained, and the file is opened. If the file cant be opened, the entry is discarded in soundmap and notification_stack, and then the loop is continued.
                                    let source = match open_audio_file(entry.path.clone()){
                                        Ok(t) => {
                                            t
                                        },
                                        Err(e) => {
                                            println!("{}",e);
                                            sound_map.remove(notification_stack[0].as_str());
                                            notification_stack.remove(0);
                                            continue
                                        }
                                    };
                                    let sink = Sink::try_new(&stream_handle).unwrap();
                                    sink.set_volume(entry.volume);
                                    sink.pause();
                                    sink.append(source);
                                    paused_notifications.push((notification_stack[0].clone(),sink));
                                    notification_stack.remove(0);
                                }
                            }

                            if queue_stack.len()>0 {
                                queue_sink.pause();
                                paused_queue.push((queue_stack[0].clone(),queue_sink));
                                queue_stack.remove(0);
                                queue_sink = Sink::try_new(&stream_handle).unwrap();

                                while queue_stack.len()>0{
                                    let entry = sound_map.get(queue_stack[0].as_str()).unwrap();
                                    let source = match open_audio_file(entry.path.clone()){
                                        Ok(t) => {
                                            t
                                        },
                                        Err(e) => {
                                            println!("{}",e);
                                            sound_map.remove(queue_stack[0].as_str());
                                            queue_stack.remove(0);
                                            continue
                                        }
                                    };
                                    let sink = Sink::try_new(&stream_handle).unwrap();
                                    sink.set_volume(entry.volume*active_notification_modifier);
                                    sink.pause();
                                    sink.append(source);
                                    paused_queue.push((queue_stack[0].clone(),sink));
                                    queue_stack.remove(0);
                                }
                            }

                        } else {
                            match sound_map.get_mut(t.2.as_str()) {
                                None => {
                                    println!("The ID does not exist.");
                                }
                                Some(k) => {
                                    match k.sound_command {
                                        SoundCommand::Play => {
                                            k.sink.pause();
                                            k.pause_state = true;
                                        }
                                        SoundCommand::Notification => {
                                            if notification_stack[0].as_str() == t.2.as_str() {
                                                notification_sink.pause();
                                                paused_notifications.push((notification_stack[0].clone(),notification_sink));
                                                notification_stack.remove(0);
                                                notification_sink = Sink::try_new(&stream_handle).unwrap();
                                            }
                                            k.pause_state = true;
                                        }
                                        SoundCommand::Queue => {
                                            if queue_stack[0].as_str() == t.2.as_str() {
                                                queue_sink.pause();
                                                paused_queue.push((queue_stack[0].clone(), queue_sink));
                                                queue_stack.remove(0);
                                                queue_sink = Sink::try_new(&stream_handle).unwrap();
                                            }
                                            k.pause_state = true;
                                        }
                                        _ => {
                                            println!("bruh, you gotta explain this");
                                        }
                                    }
                                }
                            };
                        }
                    }
                    SoundCommand::Stop => {
                        if t.2.as_str() == "" {
                            sound_map.clear();
                            paused_notifications.clear();
                            paused_queue.clear();
                            notification_stack.clear();
                            queue_stack.clear();
                            notification_sink.stop();
                            queue_sink.stop();
                        } else {
                            sound_map.remove(t.2.as_str());
                            for i in 0..notification_stack.len() {
                                if notification_stack[i].as_str() == t.2.as_str(){
                                    if i == 0 {
                                        notification_sink.stop();
                                        discard_on_load_notification = false;
                                    }
                                    notification_stack.remove(i);
                                    break
                                }
                            }
                            for i in 0..queue_stack.len() {
                                if queue_stack[i].as_str() == t.2.as_str(){
                                    if i == 0 {
                                        queue_sink.stop();
                                        discard_on_load_queue = false;
                                    }
                                    queue_stack.remove(i);
                                    break
                                }
                            }
                            for i in 0..paused_notifications.len() {
                                if paused_notifications[i].0 == t.2.as_str(){
                                    paused_notifications.remove(i);
                                }
                            }
                            for i in 0..paused_queue.len() {
                                if paused_queue[i].0 == t.2.as_str(){
                                    paused_queue.remove(i);
                                }
                            }
                        }
                    }
                    SoundCommand::Resume => {
                        if t.2.as_str() == "" {
                            for i in &mut sound_map {
                                i.1.sink.play();
                                i.1.pause_state = false;
                            }
                        } else {
                            match sound_map.get_mut(t.2.as_str()) {
                                None => {
                                    println!("The ID does not exist.");
                                }
                                Some(k) => {
                                    match k.sound_command {
                                        SoundCommand::Play => {
                                            k.sink.play();
                                            k.pause_state = false;
                                        }
                                        SoundCommand::Notification => {
                                            k.pause_state = false;
                                        }
                                        SoundCommand::Queue => {
                                            k.pause_state = false;
                                        }
                                        _ => {
                                            println!("bruh");
                                        }
                                    }
                                }
                            };
                        }
                    }
                }
            }
            Err(_) => {}
        };

        let result = queue_handling(&mut notification_sink, &mut notification_stack, &mut sound_map, &mut paused_notifications, discard_on_load_notification, 1.0, &stream_handle);
        discard_on_load_notification = result.0;
        active_notification = result.1;

        if active_notification {
            active_notification_modifier = notification_difference;
        }else {
            active_notification_modifier = 1.0;
        }
        let result = queue_handling(&mut queue_sink, &mut queue_stack, &mut sound_map,&mut paused_queue, discard_on_load_queue,active_notification_modifier, &stream_handle);
        discard_on_load_queue = result.0;
        active_queue = result.1;


        let mut delete_entries = vec![];
        //This iterates over the sound_map and checks for empty sinks.
        //Empty sinks are collected and then removed.
        //Other sinks receive a changed volume, depending on the defined volume and the notification multiplier.
        //For now it iterates over it, no matter what
        for i in &sound_map{
            match i.1.sound_command{
                SoundCommand::Play => {
                    if i.1.sink.empty(){
                        delete_entries.push(i.0.clone());
                        continue
                    }else{
                        i.1.sink.set_volume(i.1.volume*active_notification_modifier);
                    }
                }
                _ => {}
            }
        }
        for i in delete_entries{
            sound_map.remove(&*i);
        }
    }
    return_value
}

///How an entry inside the HashMap is handled, depending on whether it exists or not. Reason why discard is necessary is because an ID of a currently running soundfile could be overwritten.
///That currently running soundfile would be stopped, and entry 0 would be removed, with a changed discard, so the next 0 entry isnt discarded, whatever it may be.
fn queue_decision(values:(SoundCommand, String, String, f32), sound_map:&mut HashMap<String, AudioEntry>, queue:&mut Vec<String>, paused_queue:&mut Vec<(String, Sink)>,  queue_sink:&Sink, mut discard:bool, stream_handle:&OutputStreamHandle) -> bool{
    if sound_map.contains_key(values.2.as_str()){
        let entry = sound_map.get_mut(values.2.as_str()).unwrap();
        entry.volume = values.3;
        entry.path = values.1;
        entry.sound_command = values.0;
        //Should it be played right now, another one is pushed at the end. Should it be queued, but not played right now, its left alone.
        //Should there be no instance queued, then this entry shouldnt exist anymore!
        for i in 0..paused_queue.len(){
            if paused_queue[i].0.as_str() == values.2.as_str(){
                paused_queue.remove(i);
            }
        }

        for i in 0..queue.len() {
            if queue[i].as_str() == values.2.as_str() {
                queue.remove(i);
                if i == 0 {
                    if discard {
                        queue_sink.stop();
                        discard = false;
                    }
                }
            }
        }
        queue.push(values.2);
        //The new file is then appended and plays.
    }else{
        let sink = Sink::try_new(stream_handle).unwrap();
        let entry = AudioEntry {
            sound_command: values.0,
            volume: values.3,
            path: values.1,
            sink: sink,
            pause_state: false
        };
        //queues itself to its own sink, lol
        queue.push(values.2.clone());
        sound_map.insert(values.2, entry);
    };
    discard
}

///Handles the queue with pausable entries, etc. it returns the first bool for the current discard state, and the second bool for currently playing (in order to allow the notification volume change!)
fn queue_handling(audio_sink:&mut Sink, audio_vec:&mut Vec<String>, sound_map:&mut HashMap<String, AudioEntry>, paused_vec:&mut Vec<(String, Sink)>, mut discard: bool, volume_multiplier:f32, stream_handle:&OutputStreamHandle) -> (bool,bool){
    let mut flag = false;
    let mut playing_entry = false;

    if audio_sink.empty(){
        //This checks whether the audio_vec isnt empty, and if not, whether it was the last played soundfile. if yes, its discarded for good measure.
        if audio_vec.len()>0 {
            if discard {
                sound_map.remove(audio_vec[0].as_str());
                audio_vec.remove(0);
            }//at this point the discard is still true!
        }

        //next, the paused_notifications are checked. If a sound was paused, but then stopped, it should have been removed via stop aleady, so that case shouldnt need to be handled here
        for mut i in 0..paused_vec.len() {
            //In the scenario that the sink is inherently empty to begin with, the notification is immediately removed with the sound_map entry. The iterator is then set back by 1, and the loop is continued.
            if paused_vec[i].1.empty(){
                sound_map.remove(paused_vec[i].0.as_str());
                paused_vec.remove(i);
                i = i-1;
                continue
            }
            //
            let sound_entry = sound_map.get(paused_vec[i].0.as_str()).unwrap();
            //if the state is now unpaused, the notification should be played next. The loop for the other paused_notifications is then broken.
            if sound_entry.pause_state == false {
                //whether it already plays or not doesnt matter, if its empty, it would have been removed already.
                paused_vec[i].1.set_volume(sound_entry.volume*volume_multiplier);
                paused_vec[i].1.play();
                discard = false;
                flag = true;
                playing_entry = true;
                break;
            }else{
                continue
            }
            //okay, we have a discard bool and a audio_vec, with an empty notification_sink, but a playing paused_notification. what now?
            //during the next loop, the notification_sink will be empty again, but the audio_vec might or might not be empty. But either way, the audio_vec[0] has not been loaded into sink yet.
            //so we set the discard_on_load_notification to false.
        }
        //However, should there be no paused_notifications, or no paused_notifications to be played, the discard_bool is now true.
        //So, because we may or may not have removed an entry before, regardless, the length needs checked again. suppose, there is still a new audio_vec entry.

        //First we check whether the flag has been set because a paused notification is playing, no? great.
        if !flag {
            while audio_vec.len()>0{
                //Great, we removed the old audio_vec entry, checked for paused notifications, and still have an entry. This means, we can play the next entry!
                //well, now we get into a small pickle. we have to check whether it is supposed to be paused!
                let entry = sound_map.get(audio_vec[0].as_str()).unwrap();
                //Whether it is paused or not doesnt matter, its loaded into the sink first anyway.
                let source = match open_audio_file(entry.path.clone()){
                    Ok(t) => {
                        t
                    },
                    Err(e) => {
                        println!("{}",e);
                        //Because it failed, the entry is removed, here, as well as from sound_map!
                        sound_map.remove(audio_vec[0].as_str());
                        audio_vec.remove(0);
                        continue
                    }
                };
                //now we got a source, all thats left is to load it into the sink! No, not quite.
                //If its paused, a sink is created, the file is appended and its paused. Now it doesnt touch the notification_sink at all, it will be directly pushed to the paused_notifications!
                //After it has been pushed, the audio_vec is one entry smaller, this is why its a loop.
                if entry.pause_state {
                    let sank = Sink::try_new(stream_handle).unwrap();
                    sank.set_volume(entry.volume*volume_multiplier);
                    sank.pause();
                    sank.append(source);
                    sank.pause();
                    //test later whether an empty sink can be paused.
                    paused_vec.push((audio_vec[0].clone(), sank));
                    audio_vec.remove(0);
                }else{
                    //so, the entry was not paused, right? good. Keep in mind, the discard on empty is still true, for when this is going to be empty.
                    //the upside is, we are close to a full circle. the volume is set and the file is appended. At last, the loop is broken because we dont care about other notifications just yet.
                    audio_sink.set_volume(entry.volume*volume_multiplier);
                    audio_sink.append(source);
                    audio_sink.play();
                    playing_entry = true;
                    flag = true;
                    discard = true;
                    break
                }
            }
        }
        if !flag{
            discard = false;
        }
    }else { //this is important to implement volume changes while we are at it, in one swoop.
        let entry = sound_map.get(audio_vec[0].as_str()).unwrap();
        audio_sink.set_volume(entry.volume*volume_multiplier);
    }
    //What a journey just for checking the 2 queues. We checked if the audio_sink is empty. If it was, we checked whether the last 0 is supposed to be jettisoned, if yes, it was.
    //Then we checked for any entries in the paused queue. They all have their own very special dedicated sink. So each entry is first checked if empty. If they were empty, they would be removed. And the next one would be checked.
    //If one wasnt empty, it was checked whether it was still supposed to be paused. If it was, the next one would be checked.
    //If one wasnt empty, and it wasnt supposed to be paused, a flag is set, the discard for the next audio_vec[0] is set to false. If it was already playing, no difference.

    //The paused entries were finished to be checked. If one is playing, now the audio_vec[0] and audio_sink will be ignored, and next round the circle continues,
    // with the sink being empty, the audio_vec[0] not being discarded and the paused ones being checked again, until the currently playing one is empty and its the next ones turn.

    //If no paused entry is playing, the flag isnt set. So, while the audio_sink is not empty, the audio_vec[0] will be tried for an opened audiofile.
    // Should that fail? discard the current one manually with the sound_map entry, and onto the next one, if there is one.
    // Should there magically be no sound to be played in the audio_vec (error or amount of entries), then there is the case where the discard is still active for newly incoming audio_vec entries. We dont want that.
    // This is why, in case a file is played, the flag is set to true, for a true discard, and otherwise stays false, for no discard. So, when a newly fresh entry comes, it isnt automatically discarded and instead played!
    // Here we are, now the queues are managed. Im not going to rewrite this.
    (discard,playing_entry)
}

///Opens audio file and returns a source or returns an error if it fails.
fn open_audio_file(path: String) -> Result<Decoder<BufReader<File>>, String> {
    let audio_file = BufReader::new(match File::open(path){
        Ok(t) => {t}
        Err(e) => {
            return Err(e.to_string())
        }
    });
    let source = match Decoder::new(audio_file){
        Ok(t) => { t }
        Err(e) => {
            return Err(e.to_string())
        }
    };
    Ok(source)
}

//While technically now obsolete, Im still gonna keep it, its dope.
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