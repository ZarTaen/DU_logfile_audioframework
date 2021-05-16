# DU_logfile_audioframework

DU_logfile_audioframework is a logfile reader for Dual Universe that allows to play audiofiles via Lua Commands inside the game.

## Installation
Download the .zip and extract it to a location of your choice. Make sure the "conf" and "audiopacks" folders are in the same directory as the .exe.
Be careful about the source of the files should you get them somewhere else, as I cannot guarantee they are not tampered with.

## Usage for Endusers
In order to install Audiopacks, simply add the whole folder into the "audiopacks" directory and add an entry for the audiopack inside the "audiopacks.toml". Again, be careful where you source audiopacks from, for the same reason as above.

While there are several settings inside the "conf.toml" that can be changed, I recommend only changing the volume levels. Should an audiopack and Lua Script creator recommend more channels due to the usecase, a higher value means more compatibility and fewer delayed soundfiles. For example: audiopack1 recommends 8 channels, audiopack2 recommends 12 channels. Unless both run at the same time, 12 will always be the choice that both work with well.

Prio_count should generally not be changed without a good reason, as it changes the amount of priority levels, not the amount of priority channels, which is always 1. Anything above the existing priority level will be completely ignored.

## Usage for Creators
For supported audio formats, refer to rodio and the included audio libraries.
In general, .mp3, .wav and .flac are definitely supported.

The audiopacks are simply a folder with the audiofiles or folders with audiofiles inside. I recommend including a textfile to show how to use the paths inside the audiopacks, as well as a described usecase with a channel recommendation until a standard is established. Be very careful about including copyrighted material if you plan to distribute it, as it easily becomes a legal minefield. For actual distribution methods of audiopacks, I give no recommendations.


The lua logfile entry looks something like this:
```
system.logInfo("audioplayback|debug|sectionpass.mp3|1")
```
- The delimiter is |
- The first value decides the used functionality. For now, only audioplayback is available.
- "debug" in this example refers to the audiopacks name inside the "audiopacks" folder.
- "sectionpass.mp3" as the third value refers to the specific audiofile inside the audiopacks folder.
- The fourth entry decides the used priority level.
    - 0 stops every channel regardless of chosen audiopack or file, but the pipes are still needed.
    - 1 is for audio that is supposed to play simultanously. Preferrably for shorter sound effects like clicks or confirmation sounds.
    - 2 and above is for the priority levels of the priority queue. This is mostly for notifications and the differentiation between different importance. The current handling pauses lower priorities. 2 is a higher priority than 3 or above.

  
I recommend sound effects to have as few useless 0 volume sections at the beginning and end as possible, to allow better handling inside the queue. Should you wish to include music, never play the music in the priority queue unless notifications are supposed to play after the music track ended. These queues are shared with other scripts and audiopacks, so be considerate. In that regard I recommend allowing the player to decide the priorities.

The priority level 1 will fill the available audio channels round robin, depending on the first channel available with the lowest amount of queued sound files. This means that if music were to play in channel 1, all other channels would require to have a file playing simultanously, in order for another sound file to queue behind the music in channel 1.

For the priority queue it is important to know how priorities are handled:
Priority level 2 will always pause priority level 3+, while priority level 3 will always pause priority level 4+ and be paused by priority level 2 and so on.

It is generally possible to change the behaviour from pause to interrupt or a mix of both, but it should be standardised behaviour across the board after enough feedback, as changing it on the fly would be much harder.

## Contributing
If you want to create a fork, go ahead, don't be shy. If you want to contribute, contact me, preferably on Discord (ZarTaen#6409). Just keep in mind I have no clue what I'm, doing :D.

## TODO List
- Add option to discard specific queue (prio)
- Rework channel-system to ID based for more granular control (Pause/Stop/Resume)
- Specifying volume via lua, with a user-defined maximum.
- Adding a way to not require the audiopacks.toml
- Checking for and parsing out ../ as well as ..
- Pause/Stop/Resume per ID
- Optional Caching of Soundfiles


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