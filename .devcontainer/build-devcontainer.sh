#!/bin/bash
# This script should be executed at the very beginning to craft Docker images based on 
# the original Checkmk 1/2 Docker images which also contain Python 3.9 and Robotframework. 
# 
# 1) Edit build-devcontainer.env and change the variable CMKVERSIONS to your needs. 
#    It should only contain CMK versions you want to test/develop on.
# 2) Start build-devcontainer.sh. It will check if the CMK Docker images are already 
#    available locally. If not, it asks for credentials to download the 
#    image from the CMK download page. 
# 3) After the image tgz has been downloaded, it will be imported into Docker. 
#    (approx. 5 minutes) 
# 4) In the last step, the script will build an image based on the CMK version, including
#    Python3 and robotframework. (approx. 10 minutes)  
# $ docker images | grep mk
# robotmk-cmk-python3                                                2.0.0p5        1d96bebf47a6   27 seconds ago   2.18GB
# robotmk-cmk-python3                                                1.6.0p25       599e8beeb9c7   10 minutes ago   1.93GB


# Name of the resulting images
IMAGE=robotmk-cmk-python3
# load Checkmk versions
. build-devcontainer.env

for VERSION in $CMKVERSIONS; do
    docker images | egrep "checkmk/check-mk-enterprise.*$VERSION" 2>&1 > /dev/null
    if [ $? -gt 0 ]; then 
        echo "Docker image checkmk/check-mk-enterprise.*$VERSION is not available locally."
        read -p "Download this image? " -n 1 -r
        echo 
        if [[ $REPLY =~ ^[Yy]$ ]]; then 

            read -p "Username: " user
            DOWNLOAD_FOLDER=$(mktemp -d)
            URL=https://download.checkmk.com/checkmk/$VERSION
            TGZ=check-mk-enterprise-docker-$VERSION.tar.gz
            TGZ_FILE=${DOWNLOAD_FOLDER}/${TGZ}
            echo "+ Downloading docker image $VERSION to $DOWNLOAD_FOLDER ..."
            wget -P $DOWNLOAD_FOLDER --user $user ${URL}/${TGZ} --ask-password
            if [ -f $TGZ_FILE ]; then 
                echo "+ Importing image $TGZ_FILE ..."
                docker load -i $TGZ_FILE
            else 
                echo "ERROR: $TGZ_FILE not found!"
            fi
        else
            continue
        fi    
    fi
    echo "----"
    echo "Docker image checkmk/check-mk-enterprise.*$VERSION is ready to use"
    echo "----"
    echo "Building now the image robotmk-cmk-python3:$VERSION from Dockerfile_cmk_python ..."
    docker build -t robotmk-cmk-python3:$VERSION -f Dockerfile_cmk_python --build-arg VARIANT=$VERSION .
done