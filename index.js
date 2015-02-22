require('string.prototype.repeat')


function removePunctuation (text) {
	return text.replace(/['";:,.\/?\\-]/g, '')
}

function getSortFunctionFor (property) {

	return function (wordOne, wordTwo) {
		return wordTwo[property] - wordOne[property]
	}
}


function textalyzer (text) {

	var plainText = removePunctuation(text),
		words = plainText
			.toLowerCase()
			.trim()
			.split(/[\s\/]+/g),
		wordsCount = words.length,
		sortedWords = words.sort(),
		wordFrequenzyMap = words.reduce(
			function (previous, current, index, array) {

				previous[current] = 0

				return previous
			},
			{}
		),
		ignore = [
			'and', 'the', 'to', 'a', 'of',
			'for', 'as', 'i', 'with', 'it',
			'is', 'on', 'that', 'this', 'can',
			'in', 'be', 'has', 'if'
		],
		arr = [],
		counts = {},
		wordsSortedByFrequency,
		maximumWordFrequency,
		i


	words.forEach(function (word) {
		wordFrequenzyMap[word]++
	})

	wordsSortedByFrequency = Object
		.keys(wordFrequenzyMap)
		.map(function (word) {
			return {
				word: word,
				absoluteFrequency: wordFrequenzyMap[word],
				relativeFrequency: wordFrequenzyMap[word] / wordsCount
			}
		})
		.sort(getSortFunctionFor('absoluteFrequency'))


	maximumWordFrequency = wordsSortedByFrequency[0].absoluteFrequency

	return {
		getStats: function () {
			return {
				wordFrequency: wordsSortedByFrequency
			}
		},
		getFrequencyHistogram: function () {

			var maxWidth = 100

			return wordsSortedByFrequency
				.map(function (wordObject, index) {

					var width = Math.round(wordObject.absoluteFrequency /
					                       maximumWordFrequency * maxWidth),
						maxWordLength = 25 // TODO: get real value


					return (index + 1) + '\t' +
					       wordObject.word +
					       ' '.repeat(maxWordLength - wordObject.word.length) +
					       '■'.repeat(width) +
					       '\n'
				})
				.join('')
		}

	}
}

module.exports = textalyzer
