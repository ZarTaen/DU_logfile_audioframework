# DU_logfile_audioframework
DEPRECATED AS OF 0.28
Im still going to leave it here for everyone, because it should easily be adaptable for other usecases with logfiles.

DU_logfile_audioframework is a logfile reader for Dual Universe that allows to play audiofiles via Lua Commands inside the game.

## Installation
Put the .exe inside a folder of your choice. On first startup, the .exe will create the default conf.toml file inside the folder conf and an audiopacks folder next to the .exe, so be careful to put the program into a folder.
Be careful about the source of the files should you get them somewhere else, as I cannot guarantee they are not tampered with.
You can also take a look at the implementation of D.Mentia here:

https://github.com/Dimencia/DU-Audio-Sharp

## Usage for Endusers
The program has to be restarted if it is running while the conf.toml is changed.
In order to install Audiopacks, simply add the whole folder into the "audiopacks" directory, or wherever the creator in question thinks the files are. Again, be careful where you source audiopacks from, for the same reason as above.
You have a conf.toml file inside the conf folder that allows to change some volume settings, if you want to.

## Usage for Creators
For supported audio formats, refer to rodio and the included audio libraries.
In general, .mp3, .wav and .flac are definitely supported.
Additionally, there is an option to request playing a random file inside a folder by using a path to that folder.

The audiopacks are simply a folder with the audiofiles or folders with audiofiles inside. I recommend including a textfile to show how to use the paths inside the audiopacks, as well as a described usecase with a channel recommendation until a standard is established. Be very careful about including copyrighted material if you plan to distribute it, as it easily becomes a legal minefield. For actual distribution methods of audiopacks, I give no recommendations.

The lua logfile entries look something like this in Standard 1.1:
```
sound_play|path_to/the.mp3(string)|ID(string)|Optional Volume(int 0-100) -- Plays a concurrent sound
sound_notification|path_to/the.mp3(string)|ID(string)|Optional Volume(int 0-100) -- Lowers volume on all other sounds for its duration, and plays overtop
sound_q|path_to/the.mp3(string)|ID(string)|Optional Volume(int 0-100) -- Plays a sound after all other queued sounds finish
sound_loop|path_to/the.mp3(string)|ID(string)|Optional Volume(int 0-100) -- Plays requested sound and ignores the ID with sound_loop until the file stopped playing. Allows proper looping of files.

-- The following use the IDs that were specified in the previous three

sound_volume|ID(string)|Volume(int 0-100)

sound_pause|Optional ID(string) -- If no ID is specified, pauses all sounds
sound_stop|Optional ID(string) -- If no ID is specified, stops all sounds
sound_resume|Optional ID(string) -- If no ID is specified, resumes all paused sounds
```

The path can be absolute or relative.

I recommend sound effects to have as few useless 0 volume sections at the beginning and end as possible, to allow better handling inside the queue.
Should you wish to include music, never play the music in a queue unless you want to clog a queue.
These queues are potentially shared with other scripts and audiopacks, so be considerate. I recommend using IDs with script unique format, ideally unique for each instance of a script!
The ID is in general the specific playback ID. So if you use that ID again, the old entries (the Queues, as well as the concurrent entries) will be changed accordingly.
This means, a queued ID would be removed from the Queue, paused or not, and then placed again at the end of the queue.
Once a sound has been played, all relevant information to the ID will be removed. No caching.

## Contributing
If you want to create a fork, go ahead, don't be shy. If you want to contribute, contact me, preferably on Discord (ZarTaen#6409). Just keep in mind I have no clue what I'm doing :D.

## TODO List
- Optional Caching of Soundfiles
- Make relative path a default option, with absolute path something the user has to enable explicitly (reduces ways to troll).

Stuff moved so fast, that a lot of the things in my old TODO were obsolete and others are already implemented.

## Changes with 1.0
Lua Commands have been standardised with the help of D.Mentia.
A lot of huge rewriting for the audio logic, to allow specific behaviour of stuff in my implementation.
The removal of the audiopacks.toml (It was a bad idea with a good intention).

## Changes with 1.1
Added sound_loop (extension of the standard, with D.Mentia)
Added the randomized functionality: Instead of specifying a filepath, you can now specify a folder path and an audiofile will be chosen at random.

## Video of it in Action
https://cdn.hyperion-corporation.de/userstorages/cafa3fd0-e7cd-4a50-84a5-552f0b731fcb/music_demonstration.mp4

## Additional Clarification
Some have asked me to disclose that I am part of the Hyperion Corporation, so there you have it.

## License
MIT License

Copyright (c) 2021 ZarTaen

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
