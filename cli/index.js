#! /usr/bin/env node

var fs = require('fs'),
	path = require('path'),
	textalyzer = require('../index.js'),
	filePath = process.argv[2]


if (filePath) {
	if (!path.isAbsolute(filePath))
		filePath = path.join(process.cwd(), filePath)

	fileContent = fs.readFileSync(filePath)

	console.log(textalyzer(fileContent).getFrequencyHistogram())
}
else
	console.log('Usage: textalyzer <input-file>')
