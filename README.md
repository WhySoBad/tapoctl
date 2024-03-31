# tapoctl

Control your tapo light bulbs from anywhere on your local network using the command line

## Motivation and idea

I wanted to control my tapo light bulbs from my local network but without the necessity of granting them access to my local network (or to the internet at all).
Additionally, I wanted to be able to control by light bulbs from the command line.

The idea behind this project is to create a gRPC [server](#server) which can be hosted on a local device (see [example setup](/docs/example.md)) and which is connected 
to both your local network and the network containing the light bulbs. It then acts as a proxy and allows you to control
your light bulbs from anywhere on your local network without using the proprietary app.

>[!NOTE]
> You're still able to use the proprietary app if wanted as long as your smartphone is connected to the same wifi network as your light bulbs are

Additionally, should the cli not meet your needs the use of protocol buffers allows a quick client implementation in any language.

## Features

* Cli to control your tapo light bulbs
* Offline control for your tapo light bulbs
* gRPC server which allows easy client integration

## Supported devices

Currently, the following light bulbs are supported:

* L530
* L630
* L900

For the following devices the support is coming soon™:

* L510
* L520
* L610
* Generic light bulbs with limited feature set

## Cli

The cli supports the following commands:

* `devices`: List all devices registered on the server
* `events`: Subscribe to live events (device change, auth change) 
* `set <device>`: Update one or more properties of a light bulb <br>
    `--brightness`: Brightness value between 1 and 100<br>
    `--hue`: Hue value between 1 and 360<br>
    `--saturation`: Saturation value between 1 and 100<br>
    `--temperature`: Set color temperature to value between 2500K and 6500K<br>
    `--color`: Set predefined google home color<br>
    `--power`: Boolean whether to turn the lamp on/off
* `info <device>`: Print current state of a light bulb
* `usage <device>`: Print energy and time usage information for a light bulb
* `on <device>`: Turn light bulb on
* `off <device>`: Turn light bulb off
* `reset <device>`: Reset light bulb to factory defaults
* `serve`: Start the gRPC server. More about this can be read in [the server section](#server) <br>
    `--port`: Port on which the server should listen

Additionally, there are some global arguments which work on all commands:
* `--config`: Path to the configuration file which should be used
* `--json`: Print the response from the server as json should there be one
* `--address`: Address used for connecting to the gRPC server
* `--port`: Port used for connecting to the gRPC server
* `--secure`: Use https instead of http to connect to the gRPC server

### Configuration

By default, the configuration file is expected to be at `$HOME/.config/tapoctl/config.toml`. There are two different configuration formats: the **client** and the **server** configuration.

The client configuration is used to persist options for connecting to a server whilst the server configuration is used to register devices on the server. The server configuration is documented [in the server section](#configuration-1) in detail.

The following configuration is an example of a **client** configuration:
```toml
# Connect to the server located at `10.10.10.10`
address="10.10.10.10"
# Connect on port `19991` instead of default `19191`
port=19991
# Use http as communication protocol
secure=false
```

The client configuration is optional and when not specified otherwise everything falls back to default values

## Server

The binary includes a gRPC server which can be started using `serve` command.

The server has to be in the same network as the devices. Should your devices be in another network than your local network (e.g. guest or iot network)
you'll have to make sure the device on which the server is hosted is connected to both your local network and the network in which the lamps are located in order for you to be
able to control the bulbs from your local network. More about this can be seen in the [example setup](/docs/example.md) with a Raspberry Pi.

### Configuration

Unlike the client configuration the server configuration is required for a server to be started. It contains your tapo account credentials. Those are needed for 
constructing the secret needed for interacting with the lamps. Under no circumstances are those credentials used to connect to any (remote) tapo servers.

Additionally, you need to register your devices which then will be accessible through the gRPC api.

```toml
# Your tapo account credentials
[auth]
username=""
password=""

# Register a device with the name `lamp-1`
[devices.lamp-1]
type="L530" # The device type of the light bulb (L530, L520, ...)
address="10.255.255.10" # The address under which the device can be reached

port=19191 # Optional port to listen on. Default: 19191
timeout=5000 # Optional timeout for requests to the tapo api in milliseconds. Default: 5000
```

>[!TIP]
> You can find the ip address of your device in the official tapo app or through a
> [arp scan](https://linux.die.net/man/1/arp-scan) on the network your device is on
>
> When you use something like a pihole and assigned a hostname to your device you can also specify the hostname
> in the `address` field

### Using docker

The server can easily be set up using the `ghcr.io/whysobad/tapoctl` docker image. Since you want to communicate with your light bulbs over the network
the container needs host network access:

```bash
docker run \
  --network host \
  --volume ./config.toml:/home/tapo/.config/tapoctl/config.toml \
  ghcr.io/whysobad/tapoctl
```

When using docker compose the following configuration can be used:

```yaml
version: '3.8'

services:
  tapoctl:
    image: ghcr.io/whysobad/tapoctl
    network_mode: host
    volumes:
      - ./config:/home/tapo/.config/tapoctl/config.toml
```