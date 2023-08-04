# Ensure the python environment is created outside of mounted directory
# Otherwise, virtualbox might crash
python -m pip install poetry
poetry config virtualenvs.path "C:\Users\vagrant\Documents\"
poetry config virtualenvs.in-project false

poetry config virtualenvs.options.always-copy false
poetry install -C "C:\robotmk\"
