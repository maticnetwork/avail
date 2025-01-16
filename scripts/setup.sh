#!/bin/bash

BUILD_COMMIT=$1
LC_TAG=$2
VAL_COUNT=$3
NODE_COUNT=$4
LC_COUNT=$5

## Installing prerequisites.

curl https://sh.rustup.rs -sSf | sh -s -- -y
source $HOME/.cargo/env
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly

## Cloning and Building avail-node and light client bootstrap binaries.

git clone https://github.com/kaustubhkapatral/avail.git ~/avail-node && cd ~/avail-node
git checkout $BUILD_COMMIT
cargo build --release -p avail-node
sudo cp target/release/avail-node /usr/bin
cd $HOME
git clone https://github.com/availproject/avail-light-bootstrap.git ~/light-bootstrap && cd ~/light-bootstrap
cargo build --release
sudo cp target/release/avail-light-bootstrap /usr/bin
cd $HOME

## Downloading light client binary based on system architecture.

export aarch=$(uname -m)
if [ "$aarch" == "x86_64" ]
then
    wget https://github.com/availproject/avail-light/releases/download/$LC_TAG/avail-light-linux-amd64.tar.gz 
    tar -xvf avail-light-linux-amd64.tar.gz
    sudo mv avail-light-linux-amd64 /usr/bin/avail-light
    rm avail-light-linux-amd64.tar.gz
fi

if [ "$aarch" == "aarch64" ]
then
    wget https://github.com/availproject/avail-light/releases/download/$LC_TAG/avail-light-linux-aarch64.tar.gz 
    tar -xvf avail-light-linux-aarch64.tar.gz
    sudo mv avail-light-linux-aarch64 /usr/bin/avail-light 
    rm avail-light-linux-aarch64.tar.gz
fi

## Generating keys for validators, sudo and tech committee.

mkdir $HOME/avail-keys

for (( i=1; i<=$VAL_COUNT; i++ ))
do 
    echo "validator-$i" >> $HOME/avail-keys/nodecount.txt
done

echo "election-01" >> $HOME/avail-keys/nodecount.txt
echo "sudo-01" >> $HOME/avail-keys/nodecount.txt
echo "tech-committee-01" >> $HOME/avail-keys/nodecount.txt
echo "tech-committee-02" >> $HOME/avail-keys/nodecount.txt
echo "tech-committee-03" >> $HOME/avail-keys/nodecount.txt 
echo "load-test" >> $HOME/avail-keys/nodecount.txt

cat $HOME/avail-keys/nodecount.txt | while IFS= read -r node_name; do
    printf 'Generating keys for %s\n' "$node_name"
    avail-node key generate --output-type json --scheme Sr25519 -w 21 > $HOME/avail-keys/$node_name.wallet.sr25519.json
    cat $HOME/avail-keys/$node_name.wallet.sr25519.json | jq -r '.secretPhrase' > $HOME/avail-keys/$node_name.wallet.secret
    avail-node key generate-node-key 2> $HOME/avail-keys/$node_name.public.key 1> $HOME/avail-keys/$node_name.private.key
    avail-node key inspect --scheme Ed25519 --output-type json $HOME/avail-keys/$node_name.wallet.secret > $HOME/avail-keys/$node_name.wallet.ed25519.json
done

cd $HOME/scripts
python3 consolidate-keys.py $HOME/avail-keys

## Generating dynamic spec.

avail-node build-spec --disable-default-bootnode --chain dev > $HOME/avail-keys/devnet.template.json
python3 update-dev-chainspec.py $HOME/avail-keys
avail-node build-spec --chain=$HOME/avail-keys/populated.devnet.chainspec.json --raw --disable-default-bootnode > $HOME/avail-keys/populated.devnet.chainspec.raw.json
CHAIN_NAME=$(cat $HOME/avail-keys/populated.devnet.chainspec.raw.json | jq -r .id)

## Importing respective validator keys into their directories.

mkdir -p $HOME/avail-home/avail-validators

for (( i=1; i<=$VAL_COUNT; i++ ))
do 
    mkdir -p $HOME/avail-home/avail-validators/validator-$i/chains/$CHAIN_NAME/network
    mkdir -p $HOME/avail-home/avail-fullnodes/node-$i/chains/$CHAIN_NAME/network
    cp $HOME/avail-keys/validator-$i.private.key $HOME/avail-home/avail-validators/validator-$i/chains/$CHAIN_NAME/network/secret_ed25519
    avail-node key insert --base-path $HOME/avail-home/avail-validators/validator-$i --chain $HOME/avail-keys/populated.devnet.chainspec.raw.json --scheme Sr25519 --suri "$(cat $HOME/avail-keys/validator-${i}.wallet.secret)" --key-type babe
    avail-node key insert --base-path $HOME/avail-home/avail-validators/validator-$i --chain $HOME/avail-keys/populated.devnet.chainspec.raw.json --scheme Ed25519 --suri "$(cat $HOME/avail-keys/validator-${i}.wallet.secret)" --key-type gran
    export NODE_KEY=$(cat $HOME/avail-keys/validator-$i.public.key)
    DIFF=$(($i - 1))
    INC=$(($DIFF * 2))
    P2P=$((30335 + $INC))
    echo "--bootnodes=/ip4/127.0.0.1/tcp/$P2P/p2p/$NODE_KEY" >> $HOME/avail-keys/bootnode.txt
done

## Generating systemd service files for validators and starting the service.

export IP=$(curl ifconfig.me)

for (( i=1; i<=$VAL_COUNT; i++ ))
do
    DIFF=$(($i - 1))
    INC=$(($DIFF * 2))
    RPC=$((26657 + $INC))
    P2P=$((30335 + $INC))
    PROM=$((6000 + $INC))
    echo "[Unit]
    Description=Avail val ${i} daemon
    After=network.target
    [Service]
    Type=simple
    User=$USER
    ExecStart=$(which avail-node) --validator --allow-private-ipv4 --base-path $HOME/avail-home/avail-validators/validator-$i --rpc-port $RPC --port $P2P --prometheus-port $PROM --no-mdns --chain $HOME/avail-keys/populated.devnet.chainspec.raw.json $(cat $HOME/avail-keys/bootnode.txt) 
    Restart=on-failure
    RuntimeMaxSec=1d
    RestartSec=3
    LimitNOFILE=4096
    [Install]
    WantedBy=multi-user.target" | sudo tee "/etc/systemd/system/avail-val-${i}.service"

    sudo systemctl enable avail-val-${i}.service
    sudo systemctl start avail-val-${i}.service
    echo "Validator $i RPC endpoint is: http://$IP:$RPC" >> $HOME/endpoints.txt
    echo "Validator $i WS endpoint is : ws://$IP:$RPC" >> $HOME/endpoints.txt
done

## Generating systemd service files for full nodes and starting the service.

for (( i=1; i<=$NODE_COUNT; i++ ))
do
    mkdir -p $HOME/avail-home/avail-fullnodes/node-$i/chains/$CHAIN_NAME/network
    DIFF=$(($i - 1))
    INC=$(($DIFF * 2))
    RPC=$((9944 + $INC))
    P2P=$((30135 + $INC))
    PROM=$((6100 + $INC))
    echo "[Unit]
    Description=Avail full node daemon
    After=network.target
    [Service]
    Type=simple
    User=$USER
    ExecStart=$(which avail-node) --rpc-cors=all --rpc-port $RPC --port $P2P --prometheus-port $PROM --rpc-external --unsafe-rpc-external --no-mdns --allow-private-ipv4 --base-path $HOME/avail-home/avail-fullnodes/node-$i --chain $HOME/avail-keys/populated.devnet.chainspec.raw.json $(cat $HOME/avail-keys/bootnode.txt) 
    Restart=on-failure
    RuntimeMaxSec=1d
    RestartSec=3
    LimitNOFILE=4096
    [Install]
    WantedBy=multi-user.target" | sudo tee "/etc/systemd/system/avail-full-${i}.service"

    sudo systemctl enable avail-full-${i}.service
    sudo systemctl start avail-full-${i}.service
    echo "Fullnode ${i} RPC endpoint is: http://$IP:$RPC" >> $HOME/endpoints.txt
    echo "Fullnode ${i} WS endpoint is: ws://$IP:$RPC" >> $HOME/endpoints.txt
done

## Generate node key and config for light client bootstrap.

avail-node key generate-node-key 2> $HOME/avail-keys/light-client-boot.public.key 1> $HOME/avail-keys/light-client-boot.private.key
mkdir -p $HOME/avail-home/avail-light/light-1
echo "log_level = \"info\"
p2p_port = 39000
secret_key = { key =  \"$(cat $HOME/avail-keys/light-client-boot.private.key)\" }
identify_protocol = \"/avail_kad/id/1.0.0\"
identify_agent = \"avail-light-client/rust-client\"
kad_connection_idle_timeout = 30
kad_query_timeout = 60
avail_path = \"$HOME/avail-home/avail-light/light-1\"
" | tee "$HOME/avail-home/avail-light/light-1/config.yaml"
echo "HTTP port of bootstrap light client is: http://$IP:7000" >> $HOME/endpoints.txt

## Generate config files for light clients

for (( i=2; i<=$LC_COUNT; i++ ))
do
    mkdir -p $HOME/avail-home/avail-light/light-$i
    DIFF=$(($i - 1))
    INC=$(($DIFF * 2))
    P2P=$((37000 + $INC))
    PROM=$((9520 + $INC))
    HTTP=$((7001 + $INC))
    echo "log_level = \"info\"
    http_server_host = \"127.0.0.1\"
    http_server_port = $HTTP
    port = $P2P
    bootstraps = [[\"$(cat $HOME/avail-keys/light-client-boot.public.key)\", \"/ip4/127.0.0.1/tcp/39001\"]]
    full_node_ws = [\"ws://127.0.0.1:9944\"]
    app_id = 0
    confidence = 92.0
    prometheus_port = $PROM
    avail_path = \"$HOME/avail-home/avail-light/light-$i\" " | tee "$HOME/avail-home/avail-light/light-$i/config.yaml" 
    echo "HTTP port of light client $i is: http://$IP:$HTTP" >> $HOME/endpoints.txt
done

## Generating systemd service file for bootstrap and starting the service

echo "[Unit]
Description=Avail light client bootstrap
After=network.target
[Service]
Type=simple
User=$USER
ExecStart=$(which avail-light-bootstrap) -c $HOME/avail-home/avail-light/light-1/config.yaml
Restart=on-failure
RuntimeMaxSec=1d
RestartSec=3
LimitNOFILE=4096
[Install]
WantedBy=multi-user.target" | sudo tee "/etc/systemd/system/avail-light-1.service"
sudo systemctl enable avail-light-1.service
sudo systemctl start avail-light-1.service

## Generating systemd service files for light clients and starting the service

for (( i=2; i<=$LC_COUNT; i++ ))
do
    
    echo "[Unit]
    Description=Avail light ${i} daemon
    After=network.target
    [Service]
    Type=simple
    User=$USER
    ExecStart=$(which avail-light) -c $HOME/avail-home/avail-light/light-$i/config.yaml
    Restart=on-failure
    RuntimeMaxSec=1d
    RestartSec=3
    LimitNOFILE=4096
    [Install]
    WantedBy=multi-user.target" | sudo tee "/etc/systemd/system/avail-light-${i}.service"

    sudo systemctl enable avail-light-${i}.service
    sudo systemctl start avail-light-${i}.service
done

# Setting up the explorer
cd ~/
wget https://github.com/availproject/avail-apps/releases/download/v1.6-rc2/avail-explorer.tar.gz
tar -xvf avail-explorer.tar.gz
rm avail-explorer.tar.gz
echo "window.process_env = {"\"WS_URL"\": "\"ws://$IP:9944"\"};" >> build/env-config.js
sudo cp -r build/* /var/www/html/
sudo systemctl restart apache2
echo "Explorer url is http://$IP" >> $HOME/endpoints.txt

# Setting up observability components

# Setting up Prometheus job for collecting metrics
cd ~/
cp prometheus-example.yaml prometheus.yaml
for (( i=1; i<=$VAL_COUNT; i++ ))
do
    DIFF=$(($i - 1))
    INC=$(($DIFF * 2))
    PROM=$((6000 + $INC))
    echo "  - job_name: \"validator-$i\"
    static_configs:
      - targets: [\"localhost:$PROM\"]" >>  prometheus.yaml
done

for (( i=1; i<=$NODE_COUNT; i++ ))
do
    DIFF=$(($i - 1))
    INC=$(($DIFF * 2))
    PROM=$((6100 + $INC))
    echo "  - job_name: \"full-node-$i\"
    static_configs:
      - targets: [\"localhost:$PROM\"]
      " >>  prometheus.yaml
done

sudo systemctl restart prometheus.service

# Setting up Promtail job for collecting logs

cd ~/
cp promtail-example.yaml promtail.yaml

for (( i=1; i<=$VAL_COUNT; i++ ))
do
    echo "  - job_name: val-$i-log
    journal:
      json: false
      max_age: 12h
      path: /var/log/journal
      labels:
        job: val-$i-log
    relabel_configs:
      - action: keep
        source_labels: ["__journal__systemd_unit"]
        regex: avail-val-${i}.service
      - source_labels: ["__journal__systemd_unit"]
        target_label: "systemd_unit"
        " >> promtail.yaml
done

for (( i=1; i<=$NODE_COUNT; i++ ))
do
    echo "  - job_name: full-$i-log
    journal:
      json: false
      max_age: 12h
      path: /var/log/journal
      labels:
        job: full-$i-log
    relabel_configs:
      - action: keep
        source_labels: ["__journal__systemd_unit"]
        regex: avail-full-${i}.service
      - source_labels: ["__journal__systemd_unit"]
        target_label: "systemd_unit"
        " >> promtail.yaml
done

for (( i=1; i<=$LC_COUNT; i++ ))
do
    echo "  - job_name: light-$i-log
    journal:
      json: false
      max_age: 12h
      path: /var/log/journal
      labels:
        job: light-$i-log
    relabel_configs:
      - action: keep
        source_labels: ["__journal__systemd_unit"]
        regex: avail-light-${i}.service
      - source_labels: ["__journal__systemd_unit"]
        target_label: "systemd_unit"
        " >> promtail.yaml
done

sudo systemctl restart promtail
