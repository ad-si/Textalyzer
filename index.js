function removePunctuation (text) {
	return text.replace(/['";:,.\/?\\-]/g, '')
}

function byFrequenzy (wordOne, wordTwo) {
	return wordTwo.frequency - wordOne.frequency
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
		i


	words.forEach(function (word) {
		wordFrequenzyMap[word]++
	})

	wordsSortedByFrequency = Object
		.keys(wordFrequenzyMap)
		.map(function (word) {
			return {
				word: word,
				frequency: wordFrequenzyMap[word]
			}
		})
		.sort(byFrequenzy)

	return {
		getStats: function () {
			return {
				wordFrequency: wordsSortedByFrequency
			}
		}
	}
}

module.exports = textalyzer
