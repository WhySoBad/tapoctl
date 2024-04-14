# Example setup

In this section I'll describe my tapoctl setup using a Raspberry Pi 4b. In this setup we'll create a WiFi-Hotspot on the Raspberry Pi
without internet connection. For this to work you're required to have NetworkManager installed and set up on the Pi. Additionally, you'll need
to have the `dnsmasq` and `dnsmasq-base` packages installed. The tapoctl server is hosted using docker. If you haven't set up docker on your Raspberry Pi
you'll first have to build tapoctl on your Raspberry Pi as there are no prebuilt binaries (yet). More about this in the [server section](#tapoctl-server).

>[!IMPORTANT]
> For this setup to work the Raspberry Pi needs to be connected to your local network using ethernet since 
> the WiFi interface will be used for the hotspot

## dnsmasq

dnsmasq is a DNS and DHCP server. The following configuration can be put at `/etc/dnsmasq.conf`:

```conf
# Only listen on wlan0 interface
interface=wlan0

# The location where the leases of the DHCP server are stored
dhcp-leasefile=/var/run/dnsmasq/leases
# The ip range which is used by the DHCP server to assign ip addresses
dhcp-range=10.255.255.0,10.255.255.255

# It's recommended to assign a static DHCP lease for the lamps 
# since it can be very annoying when the ip address of the lamp
# changes over time
dhcp-host=6c:5a:b0:7d:xx:xx,set:lamps,10.255.255.xxx,infinite

# Disable DNS functionality since we won't have any internet anyways. This is only for 
# keeping port 53 free for other services like pihole or AdGuard Home
port=0
```

After configuring the `dnsmasq.service` should be started/enabled

## Creating hotspot

We use NetworkManager to create the hotspot. The full example setup can be found [here](https://gist.github.com/narate/d3f001c97e1c981a59f94cd76f041140).
The example below creates a new connection named `tapoctl-hotspot` on the `wlan0` interface:

```bash
nmcli con add type wifi ifname wlan0 con-name tapoctl-hotspot autoconnect yes ssid tapoctl
nmcli con modify tapoctl-hotspot 802-11-wireless.mode ap 802-11-wireless.band bg ipv4.method shared
nmcli con modify tapoctl-hotspot wifi-sec.key-mgmt wpa-psk
nmcli con modify tapoctl-hotspot wifi-sec.psk "<password>"
nmcli con up tapoctl-hotspot
```

To disable the internet access for this connection we have to add an iptables rule which drops all packets
which go from the `wlan0` interface to the `eth0` interface. By adding the following rule to the iptables we achieve the desired behavior:

```bash
sudo iptables -I FORWARD -i wlan0 -o eth0 -j REJECT;
```

Now you're ready to pair your light bulbs using the official Tapo app with the newly created WiFi hotspot. After entering the network credentials the Tapo app will show 
an error indicating the pairing wasn't successfully. This error can simply be ignored since the reason for the error is the Tapo app not being able to communicate with the devices
since we've put them into a network without internet access.

## tapoctl server

First, we need a tapoctl server configuration. The following can be used as a template. More about the server configurations can be found [here](../README.md#configuration-1)

```toml
[auth]
username=""
password=""

[devices.lamp-1]
type="L530"
address="10.255.255.xxx"
```

Additionally, we use the following `docker-compose.yml`:

```yaml
version: '3.8'

services:
  tapoctl:
    image: ghcr.io/whysobad/tapoctl
    network_mode: host
    volumes:
      - ./config:/home/tapo/.config/tapoctl/config.toml
```

>[!NOTE]
> In case you've compiled the tapoctl binary yourself you can start the server using `tapoctl serve -c path/to/config.toml`

After starting the server you'll see your devices using:
```bash
tapoctl devices --address <ip of Pi on LAN interface>
```