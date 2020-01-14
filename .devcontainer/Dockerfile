
FROM checkmk/check-mk-enterprise:1.6.0p6

RUN apt-get update && apt-get -y install git python-pip ipython 
#locales
ENV LANG C.UTF-8
ENV LC_ALL C.UTF-8
# RUN locale-gen de_DE.utf8
# ENV LANG de_DE.utf8
# ENV LANGUAGE de_DE:de
# ENV LC_ALL de_DE.utf8
#CMD /docker-entrypoint.sh