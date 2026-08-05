

Programar activaciones automáticas de [SelfControl](https://github.com/SelfControlApp/selfcontrol), similar a [andreasgrill/auto-selfcontrol](https://github.com/andreasgrill/auto-selfcontrol).


### Notas
- SelfControl requiere que el usuario instale una herramienta auxiliar (lo que requiere que el usuario ingrese su contraseña) para activarse. Esto es molesto, pero he añadido funcionalidad para que, si cancela la herramienta auxiliar, este programa intente de nuevo activar SelfControl, por lo que la ventana emergente de la herramienta auxiliar reaparece inmediatamente.

- Este programa instala agentes de inicio (launch agents), no demonios (daemons), ya que los demonios no tienen los permisos necesarios para activar SelfControl.

- Este programa activará SelfControl con la lista de bloqueo que hayas especificado en la aplicación SelfControl, pero si alguien quiere que los bloques tengan listas de bloqueo individuales, podría añadir esta función.

## Uso y cómo funciona
La CLI acepta 4 comandos:
- **--write_example_config** <br> Escribe un archivo de configuración de ejemplo en ~/.config/auto-selfcontrol-rs/config.aoml.
- **--remove_agents** <br> Elimina todos los agentes de inicio instalados por el programa. Estos se encuentran en ~/Library/LaunchAgents/ .
 - **--deploy** <br> Analiza el archivo de configuración e instala un agente de inicio que ejecutará --execute en este programa en los horarios de inicio de los bloques especificados en la configuración.
 - **--execute** <br> Si la hora actual se encuentra dentro de un bloque, activa SelfControl durante el tiempo restante hasta que finalice el bloque.
 Específicamente, si estamos dentro de un bloque y SelfControl está activo pero se desactiva en el tiempo t < fin del bloque, instala un agente de inicio temporal para llamar a --execute en este programa en el tiempo t.

Después de modificar el archivo de configuración, vuelve a desplegar con --deploy para actualizar.

El archivo de configuración contiene una ruta a la aplicación SelfControl y una ruta a la carpeta LaunchAgents. Modifícalas si las rutas del archivo de configuración de ejemplo no son correctas para tu máquina. 
## Instalación 
### Macs con Intel:

    curl -s -O -L \
    https://github.com/AlexanderDickie/auto-selfcontrol-rs/releases/download/v2/auto-selfcontrol-rs.x86_64-apple-darwin \ 
    mv auto-selfcontrol-rs.x86_64-apple-darwin auto-selfcontrol-rs 
    
    chmod +x auto-selfcontrol-rs
    
    ./auto-selfcontrol-rs --write_example_config
    
    // now edit the config at ~/.config/auto-selfcontrol-rs/config.aoml to your liking
    
    ./auto-selfcontrol-rs --deploy
    
### Macs con Apple Silicon:

    curl -s -O -L \
    https://github.com/AlexanderDickie/auto-selfcontrol-rs/releases/download/v2/auto-selfcontrol-rs.aarch64-apple-darwin \ 
    mv auto-selfcontrol-rs.aarch64-apple-darwin auto-selfcontrol-rs 
    
    chmod +x auto-selfcontrol-rs
    
    ./auto-selfcontrol-rs --write_example_config
    
    // now edit the config at ~/.config/auto-selfcontrol-rs/config.aoml to your liking
    
    ./auto-selfcontrol-rs --deploy
    
 
 ### O, cargo run/build, etc.
