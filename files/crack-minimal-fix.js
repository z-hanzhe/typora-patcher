const asar = require("asar");
const chalk = require("chalk");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const readlineSync = require("readline-sync");
const WinReg = require("winreg");
const { flipFuses, FuseV1Options, FuseVersion } = require("@electron/fuses");

// Fixed configuration (consistency principle)
const FIXED_REG_CODE = "DreamNya2026";
const FIXED_DATE = "1/1/2099";
const FIXED_IDATE = "12/1/2025";

/**
 * Generate injection code (Minimal fix based on user's working version)
 * Key fixes:
 * 1. Replace btoa() with Buffer.from() for Node.js compatibility
 * 2. Use consistent FIXED_REG_CODE instead of hardcoded values
 * 3. Extend interception endpoints
 * 4. Move registry fix to AFTER app ready (not during startup)
 */
function getInsertCode(atobMachineCode, email) {
    return `
/** Typora Activation Hook - Minimal Fix Version */
const electron = require("electron");
const { execFileSync } = require("child_process");
const crypto = require("crypto");

const FIXED_REG_CODE = "${FIXED_REG_CODE}";
const FIXED_DATE = "${FIXED_DATE}";
// Force trial to be expired to reduce local-store edge cases (see typora.log IDate/trailRemains).
const FIXED_IDATE = "${FIXED_IDATE}";
const HookDebug = false;

function writeLog(...data) {
    if (!HookDebug) return;
    try {
        const log = \`[\${new Date().toLocaleString()}] \${data.join(" ")}\\n------------------\\n\`;
        fs.appendFileSync(".\\\\Typora_Hook_Log.txt", log);
    } catch (e) {}
}

// IMPORTANT: Registry fix is delayed until app is ready (NOT during startup)
function fixRegistryLater() {
    try {
        // NOTE: Use single-backslash registry path. Double slashes will break reg.exe usage.
        const regPath = "HKCU\\\\Software\\\\Typora";
        const encodedRegCode = Buffer.from(FIXED_REG_CODE).toString("base64");
        const sLicense = encodedRegCode + "#0#" + FIXED_DATE;

        // Use execFileSync(args) to avoid quoting/escaping issues and to be robust across locales.
        execFileSync("reg", ["add", regPath, "/v", "SLicense", "/t", "REG_SZ", "/d", sLicense, "/f"], {
            windowsHide: true,
            timeout: 3000,
        });
        execFileSync("reg", ["add", regPath, "/v", "IDate", "/t", "REG_SZ", "/d", FIXED_IDATE, "/f"], {
            windowsHide: true,
            timeout: 3000,
        });
    } catch (e) {
        // Silent fail - don't crash the app
    }
}

// Hook fs module
const fsPathFrom = /resources[\\\\/]app[\\\\/]/i;
const fsPathTo = "resources\\\\app.bak\\\\";
const fsHook = {};

["readFileSync", "readFile", "statSync", "stat", "Stats", "StatsFs", "open", "openSync"].forEach((property) => {
    fsHook[property] = fs[property];
    fs[property] = function (filePath, ...args) {
        if (typeof filePath == "string" && fsPathFrom.test(filePath)) {
            const redirectPath = filePath.replace(fsPathFrom, fsPathTo);
            if (HookDebug) {
                writeLog(\`[fsHook] fs.\${property} redirect \${filePath} -> \${redirectPath}\`);
            }
            return fsHook[property].call(this, redirectPath, ...args);
        }
        if (HookDebug) writeLog(\`[fsHook] fs.\${property} \${filePath}\`);
        return fsHook[property].call(this, filePath, ...args);
    };
});

const fsPromisesHook = {};
["readFile", "open", "stat"].forEach((property) => {
    fsPromisesHook[property] = fs.promises[property];
    fs.promises[property] = async function (filePath, ...args) {
        if (typeof filePath == "string" && fsPathFrom.test(filePath)) {
            const redirectPath = filePath.replace(fsPathFrom, fsPathTo);
            if (HookDebug) {
                writeLog(\`[fsHook/Promises] fs.promises.\${property} redirect\`);
            }
            return fsPromisesHook[property].call(this, redirectPath, ...args);
        }
        if (HookDebug) writeLog(\`[fsHook/Promises] fs.promises.\${property} \${filePath}\`);
        return fsPromisesHook[property].call(this, filePath, ...args);
    };
});

// Hook crypto.publicDecrypt
const originalPublicDecrypt = crypto.publicDecrypt;
crypto.publicDecrypt = function (key, buffer) {
    if (HookDebug) {
        writeLog("-------------------------------------------");
        writeLog("[Monitor] crypto.publicDecrypt called");
        writeLog("Key:", key);
        writeLog("Buffer (Hex):", buffer.toString("hex"));
    }

    // FIX: Use consistent FIXED_REG_CODE
    return Buffer.from(
        JSON.stringify({
            deviceId: "${atobMachineCode.l}",
            fingerprint: "${atobMachineCode.i}",
            email: "${email}",
            license: FIXED_REG_CODE,
            version: "${atobMachineCode.v}",
            date: FIXED_DATE,
            type: "Typora",
        })
    );
};

// Hook network requests (AFTER app ready)
electron.app.whenReady().then(() => {
    // FIX: Execute registry fix AFTER app is ready (not during startup)
    setTimeout(() => {
        fixRegistryLater();
        // Periodic maintenance every 30 seconds (Typora may clear SLicense after renew checks).
        setInterval(fixRegistryLater, 30 * 1000);
    }, 5000);

    electron.protocol.handle("https", async (request) => {
        if (HookDebug) {
            writeLog(\`[electron.net Request] \${request.method} \${request.url}\`);
        }

        // FIX: Extended interception (use includes instead of strict equality)
        // Added deactivate and broader client API coverage for both typora.io and typora.com.cn
        const verificationEndpoints = [
            "/api/client/renew",
            "/api/client/activate",
            "/api/client/status",
            "/api/client/validate",
            "/api/client/deactivate"
        ];

        const shouldIntercept = verificationEndpoints.some(endpoint => request.url.includes(endpoint));

        if (shouldIntercept) {
            if (HookDebug) {
                writeLog(\`[Intercept] Fake activation response\`);
            }

            // FIX: Use Buffer.from instead of btoa (Node.js compatibility)
            // Added code: 0 and retry: true for complete API response format
            const encodedLicense = Buffer.from(FIXED_REG_CODE).toString("base64");
            return new Response(
                JSON.stringify({
                    success: true,
                    code: 0,
                    retry: true,
                    msg: encodedLicense,
                    status: "activated",
                    license: FIXED_REG_CODE,
                    expire_date: FIXED_DATE
                }),
                {
                    status: 200,
                    headers: { "content-type": "application/json" },
                }
            );
        }

        if (HookDebug) {
            try {
                const reqClone = request.clone();
                const reqBody = await reqClone.text();
                if (reqBody) {
                    writeLog("[electron.net Request Body]:", reqBody);
                }
            } catch { }

            const response = await electron.net.fetch(request, { bypassCustomProtocolHandlers: true });
            const resClone = response.clone();
            resClone
                .text()
                .then((resText) => {
                    writeLog(\`[electron.net Response] \${response.status} \${request.url}\`);
                    writeLog("[electron.net Response Body]:", resText.substring(0, 500));
                })
                .catch((err) => {
                    console.error("[electron.net Response Error]:", err);
                });

            return response;
        }

        return electron.net.fetch(request, { bypassCustomProtocolHandlers: true });
    });
});

// Hook winreg module to prevent SLicense from being cleared by onUnfillLicense
try {
    const winregModule = require("winreg");
    if (winregModule && winregModule.prototype && typeof winregModule.prototype.set === "function") {
        const originalSet = winregModule.prototype.set;
        winregModule.prototype.set = function(name, type, value, callback) {
            // Block attempts to clear SLicense
            if (name === "SLicense" && (!value || value === "" || value === "null" || String(value).startsWith("null"))) {
                if (HookDebug) {
                    writeLog(\`[winreg Hook] Blocked clearing SLicense, attempted value: \${value}\`);
                }
                // Return success without actually clearing
                if (typeof callback === "function") {
                    setImmediate(() => callback(null));
                }
                return;
            }
            // Allow other registry operations
            return originalSet.apply(this, arguments);
        };
        if (HookDebug) {
            writeLog("[winreg Hook] Successfully hooked winreg.prototype.set");
        }
    }
} catch (e) {
    // winreg module may not be available at this point, silently ignore
}
/** End of Activation Hook */
`;
}

// Main execution
async function main() {
    console.log(chalk.green("==== Typora Minimal Fix Patch ====\n"));

    // 1. Get installation path
    console.log(chalk.cyan("Finding Typora installation..."));
    const possiblePaths = [
        "D:\\Programs\\Typora",
        "C:\\Program Files\\Typora",
        process.env.LOCALAPPDATA + "\\Programs\\Typora"
    ];

    let Typora_Installation_Path = null;
    for (const p of possiblePaths) {
        if (fs.existsSync(path.join(p, "Typora.exe"))) {
            Typora_Installation_Path = p;
            break;
        }
    }

    if (!Typora_Installation_Path) {
        console.log(chalk.red("Typora not found in default locations"));
        console.log(chalk.cyan("Please enter Typora installation path: "));
        Typora_Installation_Path = readlineSync.question().trim().replace(/^"|"$/g, '');
    }

    console.log(chalk.green(`Found: ${Typora_Installation_Path}\n`));

    const resourcesPath = path.join(Typora_Installation_Path, "resources");
    const asarPath = path.join(resourcesPath, "app.asar");
    const appDir = path.join(resourcesPath, "app");
    const appBakDir = path.join(resourcesPath, "app.bak");
    const asarBakPath = path.join(resourcesPath, "app.asar.bak");
    const TyporaEXE = path.join(Typora_Installation_Path, "Typora.exe");
    const LaunchDistJS = path.join(appDir, "launch.dist.js");

    // 2. Get machine code
    console.log(chalk.cyan("Please enter machine code from Typora offline activation: "));
    const machineCode = readlineSync.question().trim();

    console.log(chalk.cyan("Please enter email (optional): "));
    const email = readlineSync.question().trim() || "user@example.com";

    let atobMachineCode;
    try {
        atobMachineCode = JSON.parse(Buffer.from(machineCode, 'base64').toString('utf-8'));
        console.log(chalk.yellow(`\nVersion: ${atobMachineCode.v}`));
        console.log(chalk.yellow(`Fingerprint: ${atobMachineCode.i}\n`));
    } catch (e) {
        console.log(chalk.red("Invalid machine code format"));
        return;
    }

    // 3. Close Typora
    try {
        execSync("taskkill /F /IM Typora.exe", { windowsHide: true });
        console.log(chalk.green("Closed Typora processes"));
    } catch (e) {}

    console.log(chalk.cyan("\nPress Enter to continue..."));
    readlineSync.question();

    console.log(chalk.green("\nStarting patch process...\n"));

    // 4. Extract asar
    if (!fs.existsSync(appDir)) {
        console.log(chalk.yellow("[1/5] Extracting app.asar..."));
        await asar.extractAll(asarPath, appDir);
        console.log(chalk.green("Done"));
    } else {
        console.log(chalk.gray("[1/5] Skip (app already exists)"));
    }

    // 5. Backup
    console.log(chalk.yellow("[2/5] Backing up files..."));
    if (!fs.existsSync(appBakDir)) {
        fs.cpSync(appDir, appBakDir, { recursive: true });
    }
    if (fs.existsSync(asarPath) && !fs.existsSync(asarBakPath)) {
        fs.unlinkSync(asarPath);
    } else if (fs.existsSync(asarPath)) {
        fs.unlinkSync(asarPath);
    }
    if (!fs.existsSync(TyporaEXE + ".bak")) {
        fs.copyFileSync(TyporaEXE, TyporaEXE + ".bak");
    }
    console.log(chalk.green("Done"));

    // 6. Modify Fuses
    console.log(chalk.yellow("[3/5] Modifying Electron Fuses..."));
    try {
        flipFuses(TyporaEXE, {
            version: FuseVersion.V1,
            [FuseV1Options.OnlyLoadAppFromAsar]: false,
        });
        console.log(chalk.green("Done"));
    } catch (e) {
        console.log(chalk.red("Failed: " + e.message));
    }

    // 7. Inject code
    console.log(chalk.yellow("[4/5] Injecting activation code..."));
    let content = fs.readFileSync(LaunchDistJS, "utf-8");

    const HOOK_START = "/** Typora Activation Hook";
    const HOOK_END = "/** End of Activation Hook */";

    // If already injected, replace the existing hook block to allow upgrades/fixes.
    if (content.includes(HOOK_START) && content.includes(HOOK_END)) {
        console.log(chalk.yellow("Already injected, updating hook block..."));
        const startIdx = content.indexOf(HOOK_START);
        const endIdx = content.indexOf(HOOK_END);
        const afterEndIdx = endIdx + HOOK_END.length;

        if (startIdx >= 0 && endIdx > startIdx) {
            const injectionCode = getInsertCode(atobMachineCode, email);
            content = content.slice(0, startIdx) + injectionCode + content.slice(afterEndIdx);

            // Write with UTF-8 no BOM
            const utf8NoBom = Buffer.from(content, "utf-8");
            fs.writeFileSync(LaunchDistJS, utf8NoBom);
            console.log(chalk.green("Done"));
        } else {
            console.log(chalk.red("Hook markers found but block range is invalid"));
        }
    } else {
        const match = content.match(/require\([^)]+\);/);
        if (match) {
            const insertPos = match.index + match[0].length;
            const injectionCode = getInsertCode(atobMachineCode, email);
            content = content.slice(0, insertPos) + injectionCode + content.slice(insertPos);

            // Write with UTF-8 no BOM
            const utf8NoBom = Buffer.from(content, "utf-8");
            fs.writeFileSync(LaunchDistJS, utf8NoBom);
            console.log(chalk.green("Done"));
        } else {
            console.log(chalk.red("Injection point not found"));
        }
    }

    // 8. Initialize registry
    console.log(chalk.yellow("[5/5] Initializing registry..."));
    const regKey = new WinReg({ hive: WinReg.HKCU, key: "\\Software\\Typora" });

    const encodedRegCode = Buffer.from(FIXED_REG_CODE).toString('base64');
    const targetVal = `${encodedRegCode}#0#${FIXED_DATE}`;

    try {
        await new Promise((resolve, reject) => {
            regKey.set("SLicense", WinReg.REG_SZ, targetVal, (err) => {
                if (err) reject(err);
                else resolve();
            });
        });
        // Keep trial date stable (avoid trial state "masking" license state during edge cases).
        await new Promise((resolve, reject) => {
            regKey.set("IDate", WinReg.REG_SZ, FIXED_IDATE, (err) => {
                if (err) reject(err);
                else resolve();
            });
        });
        console.log(chalk.green("Done\n"));
    } catch (err) {
        console.log(chalk.red("Failed: " + err.message + "\n"));
    }

    console.log(chalk.green("==== Patch Complete ====\n"));
    console.log(chalk.cyan("Next steps:"));
    console.log(chalk.gray("  1. Start Typora"));
    console.log(chalk.gray("  2. Use offline activation with any code like: +DREAM026#"));
    console.log(chalk.gray("  3. Disable auto-update and Chinese server in settings\n"));

    console.log(chalk.cyan("Press Enter to exit..."));
    readlineSync.question();
}

main().catch(err => {
    console.error(chalk.red("Error:"), err);
    console.log(chalk.cyan("\nPress Enter to exit..."));
    readlineSync.question();
});