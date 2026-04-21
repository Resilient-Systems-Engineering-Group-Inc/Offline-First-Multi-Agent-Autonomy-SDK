/**
 * Frontend wrapper for WASM bindings.
 * This provides a TypeScript-friendly interface for the WASM module.
 */

import init, { WasmTaskPlanner, WasmNetworkSimulator } from './pkg/wasm_bindings.js';

export interface Task {
    id: string;
    description: string;
    status: string;
    priority: number;
}

export class TaskPlanner {
    private planner: WasmTaskPlanner;

    constructor() {
        this.planner = new WasmTaskPlanner();
    }

    addTask(id: string, description: string, priority: number): void {
        this.planner.add_task(id, description, priority);
    }

    async planTasks(): Promise<Task[]> {
        const result = await this.planner.plan_tasks();
        return JSON.parse(JSON.stringify(result));
    }

    getTasks(): Task[] {
        const tasks = this.planner.get_tasks();
        return JSON.parse(JSON.stringify(tasks));
    }

    clear(): void {
        this.planner.clear();
    }

    taskCount(): number {
        return this.planner.task_count();
    }
}

export class NetworkSimulator {
    private simulator: WasmNetworkSimulator;

    constructor() {
        this.simulator = new WasmNetworkSimulator();
    }

    setConditions(latencyMs: number, bandwidthMbps: number, packetLossRate: number): void {
        this.simulator.set_conditions(latencyMs, bandwidthMbps, packetLossRate);
    }

    async simulateDelay(): Promise<void> {
        await this.simulator.simulate_delay();
    }

    simulatePacketLoss(): boolean {
        return this.simulator.simulate_packet_loss();
    }

    getLatency(): number {
        return this.simulator.latency_ms();
    }

    getBandwidth(): number {
        return this.simulator.bandwidth_mbps();
    }
}

// Initialize WASM module
export async function initWasm(): Promise<void> {
    await init();
    console.log('WASM module initialized');
}

// Demo usage
export async function demo(): Promise<void> {
    await initWasm();

    const planner = new TaskPlanner();
    planner.addTask('task-1', 'Explore zone A', 100);
    planner.addTask('task-2', 'Scan for objects', 150);
    planner.addTask('task-3', 'Return to base', 50);

    console.log(`Tasks: ${planner.taskCount()}`);

    const planned = await planner.planTasks();
    console.log('Planned tasks:', planned);

    const network = new NetworkSimulator();
    network.setConditions(50, 100, 0.01);
    await network.simulateDelay();
    console.log('Network latency:', network.getLatency(), 'ms');
}
