require('string.prototype.repeat')


function removePunctuation (text) {
  return text.replace(/['";:,.\/?\\-]/g, '')
}

function getSortFunctionFor (property) {
  return function (wordOne, wordTwo) {
    return wordTwo[property] - wordOne[property]
  }
}

function getFrequencyHistogram (options) {

  wordsSortedByFrequency = options.wordsSortedByFrequency
  maximumWordFrequency = options.maximumWordFrequency
  maximumWordLength = options.maximumWordLength
  maxWidth = options.maxWidth


  return wordsSortedByFrequency
    .map(function (wordObject, index) {

      var width = Math.round(
        wordObject.absoluteFrequency /
        maximumWordFrequency * maxWidth
      )

      return (index + 1) + '\t' +
      wordObject.word +
      ' '.repeat(maximumWordLength - wordObject.word.length) +
      '■'.repeat(width) +
      '\n'
    })
    .join('')
}

function getStats (wordsSortedByFrequency) {
  return {
    wordFrequency: wordsSortedByFrequency
  }
}


function textalyzer (text) {

  if (Buffer && Buffer.isBuffer(text))
    text = text.toString()


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
    maximumWordLength = 0,
    i


  words.forEach(function (word) {
    wordFrequenzyMap[word]++

    if (word.length > maximumWordLength)
      maximumWordLength = word.length
  })

  wordsSortedByFrequency = Object.keys(wordFrequenzyMap)
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
      return getStats(wordsSortedByFrequency)
    },
    getFrequencyHistogram: function () {
      return getFrequencyHistogram({
        wordsSortedByFrequency: wordsSortedByFrequency,
        maximumWordFrequency: maximumWordFrequency,
        maximumWordLength: maximumWordLength,
        maxWidth: 100
      })
    }
  }
}

module.exports = textalyzer
