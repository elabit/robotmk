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
# $CMK_ROBOT_IMAGE                                                2.0.0p5        1d96bebf47a6   27 seconds ago   2.18GB
# $CMK_ROBOT_IMAGE                                                1.6.0p25       599e8beeb9c7   10 minutes ago   1.93GB





REGISTRY="registry.checkmk.com"
ROOTDIR=$(dirname "$0")

# Name of the final image
CMK_ROBOT_IMAGE=robotmk-cmk-python3
# Dockerfile for the final image
DOCKERFILE_CMK_ROBOT=Dockerfile_cmk_python

# load Checkmk versions
. $ROOTDIR/build-devcontainer.env

function main() {
    cmk_registry_login
    build_images
}


function cmk_registry_login() {
    echo "Please provide your credentials to use the Checkmk Docker registry:"
    read -p "Username: " user
    read -p "Password: " password
    docker login $REGISTRY --username $user --password $password
    if [ $? -gt 0 ]; then 
        echo "⛔️  ERROR: Login failed. Exiting."
        exit 1
    else
        echo "🔐 Logged in to $REGISTRY."
    fi
}

function image_exists() {
    docker images | egrep -q "$1" 
}

function build_images() {
    # See https://github.com/docker/compose/issues/8449#issuecomment-914174592
    export DOCKER_BUILDKIT=0
    for VERSION in $CMKVERSIONS; do
        IMAGE_NAME="$REGISTRY/enterprise/check-mk-enterprise:$VERSION"
        IMAGE_PATTERN="$REGISTRY/enterprise/check-mk-enterprise.*$VERSION"
        if image_exists $IMAGE_PATTERN; then
            echo "Docker image $IMAGE_NAME is already available locally."
        else
            echo "Docker image $IMAGE_NAME is not yet available locally."
            read -p "Download this image? (y/n)" -n 1 -r
            echo 
            if [[ $REPLY =~ ^[Yy]$ ]]; then 
                # FIXME: v1 download with wget?
                docker pull $IMAGE_NAME
                if [ $? -gt 0 ]; then 
                    echo "⛔️  ERROR: Download failed. Exiting."
                    exit 1
                else
                    echo "✔️ Downloaded $IMAGE_NAME."
                fi
            else
                echo "❌  Skipping image build for Checkmk version $VERSION."
                continue
            fi    
        fi
        echo "Building now the local image $CMK_ROBOT_IMAGE:$VERSION from $DOCKERFILE_CMK_ROBOT ..."
        echo "Calling: docker build -t $CMK_ROBOT_IMAGE:$VERSION -f $ROOTDIR/$DOCKERFILE_CMK_ROBOT --build-arg VARIANT=$VERSION ."
        docker build -t $CMK_ROBOT_IMAGE:$VERSION -f $ROOTDIR/$DOCKERFILE_CMK_ROBOT --build-arg VARIANT=$VERSION .
        if [ $? -eq 0 ]; then 
            echo "✅  Docker image $CMK_ROBOT_IMAGE:$VERSION has been built."
        else 
            echo "⛔️  ERROR: Docker image $CMK_ROBOT_IMAGE:$VERSION could not be built."
        fi
        echo "----"
    done
}

main $@