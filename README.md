
Schedule automatic activations of [SelfControl](https://github.com/SelfControlApp/selfcontrol), similar to [andreasgrill/auto-selfcontrol](https://github.com/andreasgrill/auto-selfcontrol).

On activation, SelfControl requires the user to input their password in order to install a helper tool. This is pretty annoying but I've added functionality such that when this binary attempts to activate SC, if you cancel the helper tool installation popup, the binary immediately re-attempts, hence the popup reappears.

The binary will activate SelfControl with the blocklist you have specified in the app, but if someone wants individual blocklists for blocks I could add this.

## Usage + how it works
The binary accepts 3 arguments- deploy, execute, and remove_agents:

 - **deploy** <br>
The binary writes an example config file to ~/.config/auto-self-control-rs/config.json if the path doesn't exist. <br>
Checks the config is valid, then installs a launch agent (a daemon doesn't have the necessary permissions), which calls --execute on itself at the heads of the intervals specified in the configuration. 
 - **execute**
	 - If the current time falls within a block, and SelfControl is active: if SelfControl finishes at t < block tail, the binary installs a temporary launch agent to call --execute on itself at time t.
	 - If the current time falls within a block and SelfControl is not activate, the binary attempts to activate SelfControl for the duration until the block tail.
	 - For every other situation, does nothing.
 - **remove_agents**
The binary removes the main and temp launch agents it installed in /Users/{username}/Library/LaunchAgents .

## Installation 
Intel Macs:

    curl -s -O -L \
    https://github.com/AlexanderDickie/auto-selfcontrol-rs/releases/download/v1/auto-selfcontrol-rs.x86_64-apple-darwin \ 
    mv auto-selfcontrol-rs.x86_64-apple-darwin auto-selfcontrol-rs 
    
    chmod +x auto-selfcontrol-rs
    // now edit the config at ~/.config/auto-selfcontrol-rs/config.json to your liking
    ./auto-selfcontrol-rs deploy
    
Apple Silicon Macs:

    curl -s -O -L \
    https://github.com/AlexanderDickie/auto-selfcontrol-rs/releases/download/v1/auto-selfcontrol-rs.aarch64-apple-darwin \ 
    mv auto-selfcontrol-rs.aarch64-apple-darwin auto-selfcontrol-rs 
    
    chmod +x auto-selfcontrol-rs
    // now edit the config at ~/.config/auto-selfcontrol-rs/config.json to your liking
    ./auto-selfcontrol-rs deploy
    
 
  Or, cargo run/build etc.
