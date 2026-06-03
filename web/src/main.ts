import './style.css';
import init from '../pkg/schwarzschild.js';

async function main(): Promise<void> {
    await init();
}

main().catch(console.error);