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

client.on('open', async () => {
    console.log('Connected to server. Sending spawn.');

    process.stdin.on('keypress', (str, key) => {
        if (key.name === 'a') {
            console.log('Key pressed. Sending IncreaseDifficulty message.');
            client.send(JSON.stringify({
                type: "IncreaseDifficulty",
            }));
        }
        if (key.name === 'b') {
            console.log('Key pressed. Sending DecreaseDifficulty message.');
            client.send(JSON.stringify({
                type: "DecreaseDifficulty",
            }));
        }
        if (key.name === 'c') {
            console.log('Key pressed. Sending SpawnBloodMessage message.');
            rl.question('Enter value for a: ', (aInput) => {
                const a = parseFloat(aInput); // Parse to number
                console.log(`Sending SpawnBloodMessage with a=${a}`);
                client.send(JSON.stringify({
                    type: "SpawnBloodMessage",
                    text: "Test",
                    msg_visual: a,
                }));
            });
        }
        if (key.name === 'd') {
            console.log('Key pressed. Sending RemoveBloodMessage message.');
            client.send(JSON.stringify({
                type: "RemoveBloodMessage",
                text: "Test",
            }));
        }
    });

    await new Promise(resolve => setTimeout(resolve, 5000))
});

client.on('message', (message) => {
    console.log(`Received: ${message}`);
});

client.on('close', () => {
    console.log('Connection closed');
    rl.close(); // Close the readline interface
});