.PHONY: all test clean sample

BIN        = target/release
SAMPLE_CSV = examples/sample.csv
SAMPLE_GZ  = $(SAMPLE_CSV).gz
SAMPLE_OUT = examples/sample.isbd
SCHEMA     = id:INT64,name:VARCHAR,city:VARCHAR,score:INT64

all:
	cargo build --release

test: $(SAMPLE_CSV)
	cargo test --release

sample: all $(SAMPLE_CSV)
	$(BIN)/isbd-write $(SAMPLE_CSV) $(SAMPLE_OUT) --schema "$(SCHEMA)" --header --max-rows 2000

$(SAMPLE_CSV): $(SAMPLE_GZ)
	@gzip -dc $< > $@ || { rm -f $@; echo "error: failed to decompress $< into $@" >&2; exit 1; }

clean:
	cargo clean
	rm -f $(SAMPLE_CSV)
