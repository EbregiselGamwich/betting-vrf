const path = require('path');
const programDir = path.join(__dirname, '');
const idlDir = path.join(__dirname, '../frontend/program', 'idl');
const sdkDir = path.join(__dirname, '../frontend/program', 'generated');
const binaryInstallDir = path.join(__dirname, '.crates');

module.exports = {
    idlGenerator: 'shank',
    programName: 'vrf_betting',
    idlDir,
    sdkDir,
    binaryInstallDir,
    programDir,
};