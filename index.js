import { createStarryNight, all } from '@wooorm/starry-night'
import { toHtml } from 'hast-util-to-html'

import { createInterface } from 'node:readline'
import { stdin, stdout, exit } from 'node:process'

const rl = createInterface({
  input: stdin,
  output: stdout,
  terminal: false
});

let lang = null;
let code = '';

const starryNight = await createStarryNight(all)
console.log('ready')

rl.on('line', (line) => {
    if (line && line[0] != '\t') { // new lang
        lang = line;
    } else if (line[0] == "\t") { // add code
        code += line.slice(1) + '\n';
    } else if (lang && line === "") { // output code
        const s = starryNight.flagToScope(lang)
        if (!s) {
            exit(1);
        }
        console.log(toHtml(starryNight.highlight(code, s)))
        console.log("\x04") // EOT
        lang = null;
        code = '';
    } else  {
        exit(0);
    }
});
