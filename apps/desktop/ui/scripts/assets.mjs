import {copyFile,mkdir,cp} from 'node:fs/promises';
await mkdir('dist',{recursive:true});
for(const file of ['index.html','app.css']) await copyFile(`src/${file}`,`dist/${file}`);
await copyFile('src/assets/logo.png','dist/logo.png');

await cp("src/icons","dist/icons",{recursive:true});
