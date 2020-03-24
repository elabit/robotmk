all:
	find . -mindepth 1 -maxdepth 1 -type d -exec robot -d {} {} \;
	find . -name output.xml -exec python generate_mk_input.py --file {} \;
