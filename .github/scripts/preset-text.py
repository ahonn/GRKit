"""Print the descriptions of the recipe presets of the site as JSON: each
language code, with each preset ID and its text, from web/src/recipeStrings.ts."""
import json
import re
import sys

LANGUAGES = {"en": "en", "zhHans": "zh-Hans", "zhHant": "zh-Hant", "ja": "ja", "ko": "ko"}

source = open(sys.argv[1], encoding="utf-8").read()
result = {}
for block in re.finditer(r"^const (\w+)(?:: \w+)? = \{(.*?)^\};", source, re.S | re.M):
    language = LANGUAGES.get(block.group(1))
    texts = re.search(r"presetText: \{(.*?)\n  \}", block.group(2), re.S)
    if not language or not texts:
        continue
    result[language] = {
        key: value.replace("\\'", "'")
        for key, value in re.findall(r"^\s*'?([a-z0-9-]+)'?: '((?:[^'\\]|\\.)*)',", texts.group(1), re.M)
    }
json.dump(result, sys.stdout, ensure_ascii=False, indent=1)
