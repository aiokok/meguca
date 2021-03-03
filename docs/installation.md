List of installation and update commands to set up meguca on Debian buster.
__Use as a reference. Copy paste at your own risk.__
All commands assume to be run by the root user.

##Create user groups and users

groupadd ms          //ms is the username, which can be modified to www

useradd -g ms ms


root@Dre:~# visudo
ms ALL=(ALL:ALL) ALL   //root ALL=(ALL:ALL) ALL Add this line below

/*i Enter ECS :wq save*/

vim   /etc/hosts

------
      127.0.0.1       localhost 
      127.0.0.1       Dre
-------------

## Install

```bash
# Install OS dependencies
apt update
apt-get install -y build-essential pkg-config libpth-dev libavcodec-dev libavutil-dev libavformat-dev libswscale-dev libwebp-dev libopencv-dev libgeoip-dev git lsb-release wget curl sudo postgresql libssl-dev
apt-get dist-upgrade -y

# install PostgreSQL

https://www.postgresql.org/download/linux/ubuntu/


sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo apt-get update
sudo apt-get -y install postgresql

# Create users and DBS
service postgresql start
su postgres
psql -c "CREATE USER meguca WITH LOGIN PASSWORD 'meguca' CREATEDB"
createdb -T template0 -E UTF8 -O meguca meguca
exit

# Install Go
wget -O- wget "https://dl.google.com/go/$(curl https://golang.org/VERSION?m=text).linux-amd64.tar.gz" | tar xpz -C /usr/local
echo 'export PATH=$PATH:/usr/local/go/bin' >> /etc/profile
source /etc/profile

# Install Node.js
wget -qO- https://deb.nodesource.com/setup_10.x | bash -
apt-get install -y nodejs

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

To configure your current shell, run:
source $HOME/.cargo/env

# Clone and build meguca
git clone https://github.com/bakape/meguca.git meguca
cd meguca
make

# Edit instance configs
cp docs/config.json .
nano config.json

# Run meguca
./meguca
```

## Update

```bash
cd meguca

# Pull changes
git pull

# Rebuild
make

# Restart running meguca instance.
# This step depends on how your meguca instance is being managed.
#
# A running meguca instance can be gracefully reloaded by sending it the USR2
# signal.
```
