Pipfile.lock: Pipfile_template checkmk/Pipfile
	./pipenv lock

.venv: Pipfile.lock
	./pipenv sync && touch .venv
