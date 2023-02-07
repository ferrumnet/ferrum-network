import boto3
import re
import docker
import subprocess
import os

def main():
    while True:
        """ Filter images with prefix master- and return the latest pushed one """
        client = boto3.client('ecr', region_name='us-east-1')
        response = client.list_images(
            registryId='f5b0n3a3',
            repositoryName='ferrum_node',
            maxResults=1000
        )

        latest = None
        temp_tag = None

        for image in response['imageIds']:
            tag = image['imageTag']
            if re.search("^master-[0-9]+", tag):
                img = client.describe_images(
                    registryId='f5b0n3a3',
                    repositoryName='ferrum_node',
                    imageIds=[
                        {
        
                            'imageTag': tag
                        },
                    ]
                )
                pushed_at = img['imageDetails'][0]['imagePushedAt']
                if latest is None:
                    latest = pushed_at
                else:
                    if latest < pushed_at:
                        latest = pushed_at
                        temp_tag = tag

        # lets compare if the local image is the latest
        client = docker.from_env()
        local_images = client.images.pull("ferrum_node")

        # if not pull the latest image and restart the container
        subprocess.run(['docker', 'compose', 'up', '-d', cwd=self.path)

        time.sleep(3600)

if __name__ == "__main__":
    main()
