const WebSocket = require('ws');
const readline = require('readline');

const client = new WebSocket('ws://localhost:10001');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
});

readline.emitKeypressEvents(process.stdin);

if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
}

client.on('open', () => {
    console.log('Connected to server. Sending spawn.');

    let msg_visual = 0; // Initialize the value for msg_visual

    // Set up a periodic message sender
    //setInterval(() => {
    //    client.send(
    //        JSON.stringify({
    //            type: "RemoveBloodMessage",
    //            text: "Test",
    //        })
    //    );
    //    msg_visual += 1; // Increment msg_visual every 2 seconds
    //    console.log(`SFX id=${msg_visual}`);
    //    client.send(
    //        JSON.stringify({
    //            type: "SpawnBloodMessage",
    //            text: "Test",
    //            msg_visual: msg_visual,
    //        })
    //    );
    //}, 4500); // Run every 4 seconds

    process.stdin.on('keypress', (str, key) => {
        if (key.name === 'a') {
            console.log('Key pressed. Sending IncreaseDifficulty message.');
            client.send(
                JSON.stringify({
                    type: "IncreaseDifficulty",
                })
            );
        }
        if (key.name === 'b') {
            console.log('Key pressed. Sending DecreaseDifficulty message.');
            client.send(
                JSON.stringify({
                    type: "DecreaseDifficulty",
                })
            );
        }
        if (key.name === 'c') {
            console.log('Key pressed. Sending SpawnBloodMessage message.');
            rl.question('Enter text: ', (aInput) => {
                console.log(`Sending SpawnBloodMessage with a=${aInput}`);
                client.send(
                    JSON.stringify({
                        type: "SpawnBloodMessage",
                        text: aInput,
                        msg_visual: 30,
                    })
                );
            });
        }
        if (key.name === 'd') {
            console.log('Key pressed. Sending RemoveBloodMessage message.');
            rl.question('Enter text: ', (aInput) => {
                console.log(`Sending RemoveBloodMessage with a=${aInput}`);
                client.send(
                    JSON.stringify({
                        type: "RemoveBloodMessage",
                        text: aInput,
                    })
                );
            });
        }
        if (key.name === 'e') {
            console.log('Key pressed. Sending GetPlayerSpiritPosition message.');
            client.send(
                JSON.stringify({
                    type: "GetPlayerSpiritPosition",
                })
            );
        }
        if (key.name === 'f') {
            rl.question('Enter size: ', (aInput) => {
                rl.question('Enter size: ', (bInput) => {
                    console.log('Key pressed. Sending SetSpiritScale message.');
                    client.send(
                        JSON.stringify({
                            type: "SetSpiritScale",
                            size: parseInt(aInput),
                            power: parseFloat(bInput),
                        })
                    );
                });
            });
        }
    });
});

client.on('message', (message) => {
    console.log(`Received: ${message}`);
});

client.on('close', () => {
    console.log('Connection closed');
    rl.close(); // Close the readline interface
});
