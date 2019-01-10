'use strict'
const fs = require('fs')
const mkdirp = xs => {
  // for "package/abba/babba/foo.js", generate [[package], [package, abba], [package, abba, babba]],
  // then mkdirSync them, ignoring errors when the dirs already exist
  xs.split('/').slice(0, -1).reduce((acc, ys) => {
    acc.push([].concat(acc[acc.length - 1] || [], ys));
    return acc;
  }, []).map(ys => {
    try {
      fs.mkdirSync(ys.join('/'))
    } catch (_) { }
  })
}

const json = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))
Object.keys(json).map(
  filename => [mkdirp(filename), fs.writeFileSync(filename + '.html', json[filename])]
)
