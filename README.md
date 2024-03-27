# tapoctl

Control your tapo light bulbs from the command line on your local network

## Motivation and idea

I wanted to control my tapo light bulbs from my local network but without the necessity of granting them access to my local network (or to the internet at all).

The idea behind `tapoctl` is to host the included gRPC server on a local device (e.g. a raspberry pi) which is connected to the local and the iot network.
It acts as a proxy and allows you to control your light bulbs from anywhere on your local network. 

Additionally, should the cli not meet your needs the usage of protocol buffers
allows to quickly create your own client for your needs. 

[TODO: Write docs at /docs]