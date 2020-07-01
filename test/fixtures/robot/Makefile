all:
	find . -mindepth 1 -maxdepth 1 -type d -exec robot -d {} {} \;
	find . -name output.xml -printf '%P\n' -exec python generate_mk_input.py --file {} \;
