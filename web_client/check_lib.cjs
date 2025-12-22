const lib = require('@privacyresearch/libsignal-protocol-typescript');
console.log('KeyHelper keys:', Object.keys(lib.KeyHelper));
console.log('KeyHelper prototype keys:', Object.getOwnPropertyNames(lib.KeyHelper.prototype));
