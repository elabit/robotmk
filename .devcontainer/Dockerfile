# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.155.1/containers/ubuntu/.devcontainer/base.Dockerfile

ARG VARIANT="2.0.0-p4"
FROM checkmk/check-mk-enterprise:${VARIANT}
ARG PIP

ENV CMK_PASSWORD="cmk"

# install python3 on the container
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \ 
    && apt-get -y install wget build-essential libreadline-gplv2-dev libncursesw5-dev \
    libssl-dev libsqlite3-dev tk-dev libgdbm-dev libc6-dev libbz2-dev libffi-dev zlib1g-dev \
    && cd /tmp && wget https://www.python.org/ftp/python/3.9.4/Python-3.9.4.tgz \
    && tar xzf Python-3.9.4.tgz  \
    && cd Python-3.9.4 \
    && ./configure \
    && make build_all \
    && make install

# install python modules to run the Robotmk plugin in this container 
RUN pip3 install robotframework pyyaml mergedeep python-dateutil ipdb 

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends jq tree htop vim git telnet file lsyncd

# Creates the OMD site, executes post-start hook and halts before site start
COPY docker-entrypoint.d /docker-entrypoint.d
RUN /docker-entrypoint.sh /bin/true

# ADD requirements.txt /tmp/requirements.txt
# RUN PATH="/omd/sites/cmk/bin:${PATH}" \
#     OMD_ROOT="/omd/sites/cmk" \
#     /omd/sites/cmk/bin/${PIP} install -r /tmp/requirements.txt

# make the agent dir writeable from the CMK site (to link RF example tests) 
RUN chgrp -R cmk /usr/lib/check_mk_agent && chmod g+w /usr/lib/check_mk_agent


ENTRYPOINT ["/bin/bash"]