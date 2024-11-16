import { runCommand } from "./runHelper.ts";

const targetMap: Record<string, string> = {
    "riscv64": 'riscv64gc-unknown-none-elf',
    "x86_64": 'x86_64-unknown-none',
    "aarch64": 'aarch64-unknown-none-softfloat',
    "loongarch64": 'loongarch64-unknown-none'
};

export class KernelBuilder {
    arch: string;
    elfPath: string;
    binPath: string;
    rustflags: string;
    extraArgs: string[] = [];
    logLvl: string;

    constructor(arch: string, logLvl: string) {
        this.arch = arch;
        this.elfPath = `${Deno.cwd()}/target/${targetMap[arch]}/release/kernel`;
        this.binPath = `${this.elfPath}.bin`;

        this.rustflags = Deno.env.get('RUSTFLAGS') || "";
        this.logLvl = logLvl;

        if(arch == "loongarch64")
            this.extraArgs.push("-Zbuild-std=core,alloc");
    }

    buildFlags() {
        const rustflags = [
            "-Cforce-frame-pointers=yes",
            "-Clink-arg=-no-pie",
            "-Ztls-model=local-exec",
            `--cfg=root_fs="ext4_rs"`,
            '--cfg=board="qemu"',
            `--cfg=driver="kvirtio,kgoldfish-rtc,ns16550a"`
        ];

        this.rustflags += ' ' + rustflags.join(" ");
    }

    async buildElf() {
        this.buildFlags();
        console.log(this.rustflags);

        const buildProc = new Deno.Command("cargo", {
            args: [
                "build",
                "--release",
                "--target",
                targetMap[this.arch],
                ...this.extraArgs
            ],
            env: {
                ...Deno.env.toObject(),
                ROOT_MANIFEST_DIR: Deno.cwd() + "/",
                MOUNT_IMG_PATH: "mount.img",
                HEAP_SIZE: "0x0180_0000",
                BOARD: "qemu",
                RUSTFLAGS: this.rustflags,
                LOG: this.logLvl
            },
        });
        const code = await buildProc.spawn().status;
        if(!code.success) {
            console.error("Failed to build the kernel");
            Deno.exit(1);
        }
    }

    async convertBin() {
        await runCommand(`
            rust-objcopy --binary-architecture=${this.arch}
            ${this.elfPath}
            --strip-all
            -O binary
            ${this.binPath}
        `).spawn().status;
    }
}
