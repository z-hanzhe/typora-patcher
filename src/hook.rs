// The Typora activation hook JS, kept verbatim from the original Node implementation.
// Placeholders {{...}} are replaced at runtime with the user's values.

const HOOK_TEMPLATE: &str = r##"
/** Typora Activation Hook - Minimal Fix Version */
const electron = require("electron");
const { execFileSync } = require("child_process");
const crypto = require("crypto");

const FIXED_REG_CODE = "{{REG_CODE}}";
const FIXED_DATE = "{{DATE}}";
// Force trial to be expired to reduce local-store edge cases (see typora.log IDate/trailRemains).
const FIXED_IDATE = "{{IDATE}}";
const HookDebug = false;

function writeLog(...data) {
    if (!HookDebug) return;
    try {
        const log = `[${new Date().toLocaleString()}] ${data.join(" ")}\n------------------\n`;
        fs.appendFileSync(".\\Typora_Hook_Log.txt", log);
    } catch (e) {}
}

// IMPORTANT: Registry fix is delayed until app is ready (NOT during startup)
function fixRegistryLater() {
    try {
        // NOTE: Use single-backslash registry path. Double slashes will break reg.exe usage.
        const regPath = "HKCU\\Software\\Typora";
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
const fsPathFrom = /resources[\\/]app[\\/]/i;
const fsPathTo = "resources\\app.bak\\";
const fsHook = {};

["readFileSync", "readFile", "statSync", "stat", "Stats", "StatsFs", "open", "openSync"].forEach((property) => {
    fsHook[property] = fs[property];
    fs[property] = function (filePath, ...args) {
        if (typeof filePath == "string" && fsPathFrom.test(filePath)) {
            const redirectPath = filePath.replace(fsPathFrom, fsPathTo);
            if (HookDebug) {
                writeLog(`[fsHook] fs.${property} redirect ${filePath} -> ${redirectPath}`);
            }
            return fsHook[property].call(this, redirectPath, ...args);
        }
        if (HookDebug) writeLog(`[fsHook] fs.${property} ${filePath}`);
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
                writeLog(`[fsHook/Promises] fs.promises.${property} redirect`);
            }
            return fsPromisesHook[property].call(this, redirectPath, ...args);
        }
        if (HookDebug) writeLog(`[fsHook/Promises] fs.promises.${property} ${filePath}`);
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
            deviceId: "{{DEVICE_ID}}",
            fingerprint: "{{FINGERPRINT}}",
            email: "{{EMAIL}}",
            license: FIXED_REG_CODE,
            version: "{{VERSION}}",
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
            writeLog(`[electron.net Request] ${request.method} ${request.url}`);
        }

        // FIX: Extended interception (use includes instead of strict equality)
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
                writeLog(`[Intercept] Fake activation response`);
            }

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
                    writeLog(`[electron.net Response] ${response.status} ${request.url}`);
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
            if (name === "SLicense" && (!value || value === "" || value === "null" || String(value).startsWith("null"))) {
                if (HookDebug) {
                    writeLog(`[winreg Hook] Blocked clearing SLicense, attempted value: ${value}`);
                }
                if (typeof callback === "function") {
                    setImmediate(() => callback(null));
                }
                return;
            }
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
"##;

pub struct HookConfig<'a> {
    pub reg_code: &'a str,
    pub date: &'a str,
    pub idate: &'a str,
    pub device_id: &'a str,
    pub fingerprint: &'a str,
    pub email: &'a str,
    pub version: &'a str,
}

pub fn build_hook(cfg: &HookConfig) -> String {
    HOOK_TEMPLATE
        .replace("{{REG_CODE}}", cfg.reg_code)
        .replace("{{DATE}}", cfg.date)
        .replace("{{IDATE}}", cfg.idate)
        .replace("{{DEVICE_ID}}", cfg.device_id)
        .replace("{{FINGERPRINT}}", cfg.fingerprint)
        .replace("{{EMAIL}}", cfg.email)
        .replace("{{VERSION}}", cfg.version)
}

pub const HOOK_START: &str = "/** Typora Activation Hook";
pub const HOOK_END: &str = "/** End of Activation Hook */";
