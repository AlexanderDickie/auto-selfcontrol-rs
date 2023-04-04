Schedule automatic activations of [SelfControl](https://github.com/SelfControlApp/selfcontrol), similar to [andreasgrill/auto-selfcontrol](https://github.com/andreasgrill/auto-selfcontrol).


### Notes
- SelfControl requires the user to install a helper tool (requiring the user to input their password) in order to activate. This is annoying but I've added functionality such that if you cancel the helper tool, this program will reattempt to activate SelfControl, hence the helper tool popup window immediatly reappears.

- This program installs launch agents, not daemons, as daemons don't have the necessary permissions to activate SelfControl.

- This program will activate SelfControl with the blocklist you have specified in the SelfControl app, but if someone wants blocks to have individual blocklists I could add this.

## Usage + how it works
The cli accepts 4 commands:
- **- -write_example_config** <br> Writes an example configuration file to ~/.config/auto-self-control-rs/config.aoml.
- **- -remove_agents** <br> Removes all launch agents installed by the program. They live in ~/Library/LaunchAgents/ .
 - **- -deploy** <br> Parses the config file then installs a launch agent which will call - -execute on this program at the start times of the blocks specified in the config.
 - **- -execute** <br> If the current time is within a block, activates SelfControl for the duration remaining until the block ends.
 Specifically, if we are within a block and SelfControl is active but deactivates at time t < block end, installs a temporary launch agent to call - -execute on this program at time t.

After altering the configuration file, re-deploy with --deploy to update.

The config file contains a path to the SelfControl app and a path to the LaunchAgents folder. Alter these if the paths in the example config file aren't accurate for your machine. 
## Installation 
### Intel Macs:

    curl -s -O -L \
    https://github.com/AlexanderDickie/auto-selfcontrol-rs/releases/download/v2/auto-selfcontrol-rs.x86_64-apple-darwin \ 
    mv auto-selfcontrol-rs.x86_64-apple-darwin auto-selfcontrol-rs 
    
    chmod +x auto-selfcontrol-rs
    
    ./auto-selfcontrol-rs --write_example_config
    
    // now edit the config at ~/.config/auto-selfcontrol-rs/config.aoml to your liking
    
    ./auto-selfcontrol-rs --deploy
    
### Apple Silicon Macs:

    curl -s -O -L \
    https://github.com/AlexanderDickie/auto-selfcontrol-rs/releases/download/v2/auto-selfcontrol-rs.aarch64-apple-darwin \ 
    mv auto-selfcontrol-rs.aarch64-apple-darwin auto-selfcontrol-rs 
    
    chmod +x auto-selfcontrol-rs
    
    ./auto-selfcontrol-rs --write_example_config
    
    // now edit the config at ~/.config/auto-selfcontrol-rs/config.aoml to your liking
    
    ./auto-selfcontrol-rs --deploy
    
 
 ### Or, cargo run/build etc.
