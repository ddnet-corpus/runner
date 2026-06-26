#pragma once

extern "C" {
#include <ddcorpus/runner.h>
}

class Corpus {
	DDCorpCorpus *corpus;

public:
	Corpus(const char *path) :
		corpus(ddcorp_corpus_create(path)) {}

	void add_runner(const char *path, void (*runner)(const uint8_t *, uintptr_t, const char *, struct DDCorpBuffer *)) {
		ddcorp_corpus_add_runner(corpus, path, runner);
	}

	bool run() { return ddcorp_corpus_run(corpus); }

	~Corpus() { ddcorp_corpus_free(corpus); }
};
