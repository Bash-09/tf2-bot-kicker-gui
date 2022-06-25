<p align="center">
  <img src="images/logo_smol.png">
</p>

# TF2 Bot Kicker by Bash

A (somewhat) cross-platform bot identifier/kicker written in Rust.

![Demonstration Image](images/demo.png)

# What it does

This program runs while you play TF2 and is able to check players on your server against a saved list of players to check if they are a bot or cheater and can automatically perform features such as votekick them or send chat messages to other players to warn of them joining the server. It also has functionality to check against common bot names (e.g. DoesHotter) or check if an account has stolen some else's name and automatically mark those accounts as bots.

# Usage

Download the program from [here.](https://github.com/Googe14/tf2-bot-kicker-gui/releases)

```
ip 0.0.0.0
rcon_password tf2bk
net_start
```

1. Add the above 3 lines to your TF2 autoexec.cfg (You can choose anything for the rcon_password, you will just have to set it when you start the program)
2. Add `-condebug -conclearlog -usercon` to your TF2 launch options. (Right click Team Fortress 2 in your Steam library, select Properties, and paste into the Launch Options section)
3. Launch TF2.
4. Run the program and set your TF2 directory.

Next time you play TF2 you will just need to start the program and it will do everything else for you!

# Settings and Configuration

To reset your settings, delete the `settings.json` file in the `cfg` folder.

### General
* `User` - Your SteamID3 (like from when you use the status command in-game) to indentify if bots are on the friendly or enemy team. (will stop attempting to kick enemy bots if set)
* `RCon Password` - Make sure this is the same as is set by rcon_password in your autoexec.cfg file.
* `Refresh Period` - How often to refresh the info from the server
### Kicking
* `Kick Bots` - Automatically call votekicks on identified bots.
* `Kick Cheaters` - Automatically call votekicks on known cheaters.
* `Kick Period` - How many seconds between kick attempts
### Chat Messages
* `Announce Bots` - Send chat messages indicating bots joining the server.
* `Announce Cheaters` - Send chat messages indicating cheaters joining the server (If both bot and cheater announcements are enabled they will be combined into singular chat messages).
* `Announce Name-stealing` - Announce when a bot changes it's name to another player's name (will check for invisible characters in their name as well). Hopefully this is no longer needed with Valve's recent patches.
* `Chat Message Period` - Time in seconds between sending chat messages.
### Bot Detection
* `Mark accounts with a stolen name as bots` - Enable accounts that steal another player's name to be automatically marked as a bot.

### Other
Any saved regexes or players can be accessed/added/editted/deleted from the `Saved Data` tab at the top.

![Demonstration of editing a saved player](images/saved_account_demo.png)

## Account identification

A list of accounts is stored in `cfg/playerlist.json` containing the SteamID, player type (Player/Bot/Cheater) and any recorded notes for that account. When players join the server their steamid is matched against this list to determine if they are a bot or cheater and will take appropriate action (send chat messages, kick, etc). If they are not a know account their name will be checked against a list of regexes in case they have a common bot name (e.g. DoesHotter).

# Building
This program should build without issue through Cargo on Windows. 

On Linux some libraries may need to be installed. (As listed in the Ubuntu repository)

`librust-gdk-dev`\
`libudev-dev`\
`libxcb-render0-dev`\
`libxcb-shape0-dev`\
`libxcb-xfixes0-dev`

# How it works
 
By adding `-usercon` to your TF2 launch options and the settings in your `autoexec.cfg` file, programs are able to initiate a Remote CONsole with the game over a TCP connection. From this RCON connection, this program can execute commands and read the response while you are busy playing TF2. Using commands like `status` and `tf_lobby_debug`, the program is able to see the names and SteamIDs of players in your Casual server, which it uses to identify bots according to the SteamIDs or names saved in any files you have set in the program. If any players are identified as a bot by their SteamID or name, this program will take appropriate action.

Unfortunately the `status` command runs but does not respond over rcon, instead outputting into the local game console. To overcome this, `-condebug -conclearlog` is added the the TF2 launch options to output the contents of the in-game console to a log file, which this program reads from to get the output of the status command.
