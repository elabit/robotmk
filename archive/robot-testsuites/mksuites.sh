#!/bin/bash


# Level = 1 2 3
# No = A B C

CWD=$(pwd)

SUITENAME="oksuite"
mkdir -p suites/$SUITENAME
cd suites/$SUITENAME
for no in A B C; do 
	for l in 1 2 3; do 
		mkdir -p L$l-Suites
		cd L$l-Suites
		mkdir -p L$l-$no-Suites
		cd L$l-$no-Suites
		for i in 1 2 3; do
			cp $CWD/templates/$SUITENAME.robot L$l-Testfile-$no-$i.robot
		done
		cd ../..
	done
done
cd $CWD

mkdir -p results
robot -o results/$SUITENAME.xml -l NONE -r NONE suites/$SUITENAME
python xml2mkinput.py -i results/$SUITENAME.xml -o results/$SUITENAME.json
