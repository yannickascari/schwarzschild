import './style.css';

async function main(): Promise<void> {
    if (!navigator.gpu) {
        document.getElementById('no-webgpu')!.style.display = 'flex';
        return;
    }

    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
        document.getElementById('no-webgpu')!.style.display = 'flex';
        return;
    }

    const { default: init } = await import('../pkg/schwarzschild.js');
    await init();
}

main().catch(console.error);