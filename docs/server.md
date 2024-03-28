# Server

The binary includes a gRPC server which can be started using `tapoctl serve`. The server expects a configuration file.
By default, the configuration file is expected at `$HOME/.config/tapoctl/config.toml`. Using the `-c/--config` argument you can specify an
alternative config path.

The server has to be in the same network as the devices. Should your devices be in another network than your local network (e.g. guest or iot network) 
you'll have to make sure the device on which the server is run is connected to both your local network and the guest or iot network in order for you to be 
able to control the bulbs from your local network. More about this can be seen in the [example setup](/example.md) with a rasperry pi.

## Configuration

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
```

>[!TIP]
> You can find the ip address of your device in the official tapo app or through a 
> [arp scan](https://linux.die.net/man/1/arp-scan) on the network your device is on
> 
> When you use something like a pihole and assigned a hostname to your device you can also specify the hostname 
> in the `address` field

## Using docker

The server can easily be set up using the `ghcr.io/whysobad/tapoctl` docker image:

```bash
docker run --network host --volume ./config.toml:/home/tapo/.config/tapoctl/config.toml ghcr.io/whysobad/tapoctl
```

or using docker compose:

```yaml
version: '3.8'

services:
  tapoctl:
    image: ghcr.io/whysobad/tapoctl
    network_mode: host
    volumes:
      - ./config:/home/tapo/.config/tapoctl/config.toml
```