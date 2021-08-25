# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.155.1/containers/ubuntu/.devcontainer/base.Dockerfile

ARG VARIANT
# FROM checkmk/check-mk-enterprise:${VARIANT}
FROM robotmk-cmk-python3:${VARIANT}
ARG PIP

ENV CMK_PASSWORD="cmk"

# Creates the OMD site, executes post-start hook and halts before site start
COPY docker-entrypoint.d /docker-entrypoint.d
RUN /docker-entrypoint.sh /bin/true

# make the agent dir writeable from the CMK site (to link RF example tests) 
RUN chgrp -R cmk /usr/lib/check_mk_agent && chmod g+w /usr/lib/check_mk_agent


ENTRYPOINT ["/bin/bash"]