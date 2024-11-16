import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { KernelBuilder } from "./kernel.ts";
import { runCommand } from "./runHelper.ts";
import { parseArgsString } from "./runHelper.ts";

class QemuRunner {
    arch: string;
    bus: string = "device";
    builder: KernelBuilder;
    memorySize: string = "1G";
    smp: string = "1";

    constructor(options: CommandOptions<globalArgType>, builder: KernelBuilder) {
        this.arch = options.arch;
        this.builder = builder;
        if (this.arch == "x86_64" || this.arch == "loongarch64")
            this.bus = "pci";
    }

    getQemuArchExec(): string[] {
        return {
            x86_64: parseArgsString(`
                -machine q35
                -kernel ${this.builder.elfPath}
                -cpu IvyBridge-v2    
            `),
            riscv64: parseArgsString(`
                -machine virt
                -kernel ${this.builder.binPath}
            `),
            aarch64: parseArgsString(`
                -machine virt
                -cpu cortex-a72
                -kernel ${this.builder.binPath}
            `),
            loongarch64: parseArgsString(`
                -kernel ${this.builder.elfPath}
            `)
        }[this.arch] ?? [];
    }

    async run() {
        await runCommand(`
            qemu-system-${this.arch} ${this.getQemuArchExec().join(" ")}
            -m ${this.memorySize}
            -smp ${this.smp}
            -D qemu.log",
            -d in_asm,int,pcall,cpu_reset,guest_errors
            -drive
            file=mount.img,if=none,format=raw,id=x0
            -device
            virtio-blk-${this.bus},drive=x0
            -nographic
        `).spawn().status;
    }
}

async function runQemu(options: CommandOptions<globalArgType>) {
    const builder = new KernelBuilder(options.arch, options.logLevel);
    await builder.buildElf();
    await builder.convertBin();

    const runner = new QemuRunner(options, builder);
    await runner.run();
}

export const cliCommand = new Command<globalArgType>()
    .description("Run kernel in the qemu")
    .action(runQemu);
