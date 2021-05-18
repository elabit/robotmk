# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.155.1/containers/ubuntu/.devcontainer/base.Dockerfile

ARG VARIANT="2.0.0-p4"
FROM checkmk/check-mk-enterprise:${VARIANT}
ARG PIP

ENV CMK_PASSWORD="cmk"
# Creates the OMD site but halts before site start
RUN /docker-entrypoint.sh /bin/true

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends jq tree htop vim git

ADD requirements.txt /tmp/requirements.txt
RUN PATH="/omd/sites/cmk/bin:${PATH}" \
    OMD_ROOT="/omd/sites/cmk" \
    /omd/sites/cmk/bin/${PIP} install -r /tmp/requirements.txt

ENTRYPOINT ["/bin/bash"]