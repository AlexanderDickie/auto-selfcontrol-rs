Schedule activations of SelfControl https://github.com/SelfControlApp/selfcontrol - similar to https://github.com/andreasgrill/auto-selfcontrol.



SelfControl requires the user to install a helper tool, which is annoying when you want to activate it automatically. But I've added functionality such that if SelfControl is executed by this binary, if the user then cancels the helper, self control is immedialty re-executed and the helper installation window reappears.

How it works-
This binary has 3 valid arguments- deploy, execute, remove_agents.

deploy- Writes an example config to ~/.config/self-control-rs/config.json if the path doesn't exist. Installs a launch agent which calls execute on this same binary according to the schedule in config.json.

execute- if the current time falls within a block in the config, executes SelfControl for remaining duration to the end of the current block.
