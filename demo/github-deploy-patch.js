/* this is just needed to patch the github pages deployment
 * because github pages does not support relative paths
 */
const BASENAME = "awsm-renderer";
const ROOTS = ["css", "demo", "media"];

// nothing more to config past here
const fs = require("fs");

(async () => {
    let data = await readAsync("./dist/index.html");
    for(const root of ROOTS) {
        data = data.replaceAll(`"/${root}`, `"/${BASENAME}/${root}`);
        data = data.replaceAll(`'/${root}`, `'/${BASENAME}/${root}`);
    }

    await writeAsync("./dist/index.html", data);
    await writeAsync("./dist/404.html", data);
})();

function readAsync(src) {
    return new Promise((resolve, reject) => {
        fs.readFile(src, "utf-8", (err, data) => {
            if(err) {
                reject(err);
            } else {
                resolve(data);
            }
        });
    });
}

function writeAsync(dest, data) {
    return new Promise((resolve, reject) => {
        fs.writeFile(dest, data, { encoding: "utf8", flag: "w", }, (err) => {
            if(err) {
                reject(err);
            } else {
                resolve();
            }
        });
    });
}