B_FILES = $(shell find . -name '*.b')
BF_FILES = $(shell find . -name '*.bf')
IR_FILES_B = $(B_FILES:%.b=%.ir)
IR_FILES_BF = $(BF_FILES:%.bf=%.ir)
BFRS = ../../target/release/bfrs

.PHONY: all
all: compile $(IR_FILES_B) $(IR_FILES_BF)

.PHONY: compile
compile:
	cargo build --release

$(IR_FILES_B): %.ir: %.b
	$(BFRS) $< > $@
$(IR_FILES_BF): %.ir: %.bf
	$(BFRS) $< > $@

.PHONY: clean
clean:
	rm -f $(IR_FILES_B) $(IR_FILES_BF)
